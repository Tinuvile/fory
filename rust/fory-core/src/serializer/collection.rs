// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::error::Error;
use crate::resolver::context::ReadContext;
use crate::resolver::context::WriteContext;
use crate::serializer::{ForyDefault, Serializer};
use crate::types::PRIMITIVE_ARRAY_TYPES;

// const TRACKING_REF: u8 = 0b1;

pub const HAS_NULL: u8 = 0b10;

// Whether collection elements type is declare type.
const DECL_ELEMENT_TYPE: u8 = 0b100;

//  Whether collection elements type same.
pub const IS_SAME_TYPE: u8 = 0b1000;

pub fn write_collection_type_info(
    context: &mut WriteContext,
    is_field: bool,
    collection_type_id: u32,
) {
    if is_field {
        return;
    }
    context.writer.write_varuint32(collection_type_id);
}

pub fn write_collection<'a, T: Serializer + 'a, I: IntoIterator<Item = &'a T>>(
    iter: I,
    context: &mut WriteContext,
    is_field: bool,
) {
    let items: Vec<&T> = iter.into_iter().collect();
    let len = items.len();
    context.writer.write_varuint32(len as u32);
    if len == 0 {
        return;
    }
    let mut header = 0;
    let mut has_null = false;
    if T::fory_is_option() {
        for item in &items {
            if item.fory_is_none() {
                has_null = true;
                break;
            }
        }
    }
    let is_same_type = !T::fory_is_polymorphic();
    if has_null {
        header |= HAS_NULL;
    }
    if is_field {
        header |= DECL_ELEMENT_TYPE;
    }
    if is_same_type {
        header |= IS_SAME_TYPE;
    }
    context.writer.write_u8(header);
    T::fory_write_type_info(context, is_field);
    // context.writer.reserve((T::reserved_space() + SIZE_OF_REF_AND_TYPE) * len);
    if T::fory_is_polymorphic() || T::fory_is_shared_ref() {
        // TOTO: make it xlang compatible
        for item in &items {
            item.fory_write(context, is_field);
        }
    } else {
        // let skip_ref_flag = crate::serializer::get_skip_ref_flag::<T>(context.get_fory());
        let skip_ref_flag = is_same_type && !has_null;
        for item in &items {
            crate::serializer::write_ref_info_data(*item, context, is_field, skip_ref_flag, true);
        }
    }
}

pub fn read_collection_type_info(
    context: &mut ReadContext,
    is_field: bool,
    collection_type_id: u32,
) {
    if is_field {
        return;
    }
    let remote_collection_type_id = context.reader.read_varuint32();
    assert_eq!(collection_type_id, remote_collection_type_id);
    if PRIMITIVE_ARRAY_TYPES.contains(&remote_collection_type_id) {
        panic!("Vec<number> belongs to the `number_array` type, and Vec<Option<number>> belongs to the `list` type. You should not read data of type `number_array` as data of type `list`");
    }
    assert_eq!(remote_collection_type_id, collection_type_id);
}

pub fn read_collection<C, T>(context: &mut ReadContext) -> Result<C, Error>
where
    T: Serializer + ForyDefault,
    C: FromIterator<T>,
{
    let len = context.reader.read_varuint32();
    if len == 0 {
        return Ok(C::from_iter(std::iter::empty()));
    }
    let header = context.reader.read_u8();
    let declared = (header & DECL_ELEMENT_TYPE) != 0;
    T::fory_read_type_info(context, declared);
    let has_null = (header & HAS_NULL) != 0;
    let is_same_type = (header & IS_SAME_TYPE) != 0;
    if T::fory_is_polymorphic() || T::fory_is_shared_ref() {
        (0..len)
            .map(|_| T::fory_read(context, declared))
            .collect::<Result<C, Error>>()
    } else {
        let skip_ref_flag = is_same_type && !has_null;
        // let skip_ref_flag = crate::serializer::get_skip_ref_flag::<T>(context.get_fory());
        (0..len)
            .map(|_| crate::serializer::read_ref_info_data(context, declared, skip_ref_flag, true))
            .collect::<Result<C, Error>>()
    }
}

/// Read collection data directly into an existing collection, avoiding allocation.
/// This is optimized for reusing collections by clearing and refilling them.
pub fn read_collection_into<C, T>(context: &mut ReadContext, output: &mut C) -> Result<(), Error>
where
    T: Serializer + ForyDefault,
    C: Extend<T> + Clear,
{
    let len = context.reader.read_varuint32();

    // Clear existing collection but retain capacity
    output.clear();

    if len == 0 {
        return Ok(());
    }

    let header = context.reader.read_u8();
    let declared = (header & DECL_ELEMENT_TYPE) != 0;
    T::fory_read_type_info(context, declared);
    let has_null = (header & HAS_NULL) != 0;
    let is_same_type = (header & IS_SAME_TYPE) != 0;

    if T::fory_is_polymorphic() || T::fory_is_shared_ref() {
        let elements = (0..len)
            .map(|_| T::fory_read(context, declared))
            .collect::<Result<Vec<T>, Error>>()?;
        output.extend(elements);
    } else {
        let skip_ref_flag = is_same_type && !has_null;
        let elements = (0..len)
            .map(|_| crate::serializer::read_ref_info_data(context, declared, skip_ref_flag, true))
            .collect::<Result<Vec<T>, Error>>()?;
        output.extend(elements);
    }

    Ok(())
}

/// Trait for collections that can be cleared while retaining their capacity.
/// This is used by `read_collection_into` to optimize memory usage.
pub trait Clear {
    fn clear(&mut self);
}

// Implement Clear for standard library collections
impl<T> Clear for Vec<T> {
    fn clear(&mut self) {
        self.clear();
    }
}

impl<T> Clear for std::collections::HashSet<T> {
    fn clear(&mut self) {
        self.clear();
    }
}

impl<K, V> Clear for std::collections::HashMap<K, V> {
    fn clear(&mut self) {
        self.clear();
    }
}

impl<K, V> Clear for std::collections::BTreeMap<K, V> {
    fn clear(&mut self) {
        self.clear();
    }
}
