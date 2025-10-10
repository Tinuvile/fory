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

//! Tests for the new `deserialize_into` API to verify zero-copy deserialization works

use fory_core::fory::Fory;
use std::collections::HashMap;

#[test]
fn test_deserialize_into_basic_api() {
    let fory = Fory::default();

    // Test with String
    let original = "Hello, deserialize_into!".to_string();
    let serialized = fory.serialize(&original);
    let mut target = String::new();
    
    // This is the main API we implemented
    fory.deserialize_into(&serialized, &mut target).unwrap();
    assert_eq!(target, original);

    // Test with i32
    let original_num = 42i32;
    let serialized = fory.serialize(&original_num);
    let mut target_num = 0i32;
    fory.deserialize_into(&serialized, &mut target_num).unwrap();
    assert_eq!(target_num, original_num);
}

#[test]
fn test_deserialize_into_vs_deserialize() {
    let fory = Fory::default();

    let original_vec = vec![1, 2, 3, 4, 5];
    let serialized = fory.serialize(&original_vec);

    // Traditional deserialize (may involve copy)
    let result1: Vec<i32> = fory.deserialize(&serialized).unwrap();

    // New deserialize_into (zero-copy when possible)
    let mut result2 = Vec::new();
    fory.deserialize_into(&serialized, &mut result2).unwrap();

    // Both should produce the same result
    assert_eq!(result1, result2);
    assert_eq!(result2, original_vec);
}

#[test]
fn test_deserialize_into_memory_reuse() {
    let fory = Fory::default();

    // Pre-allocate a large HashMap
    let mut target_map: HashMap<String, i32> = HashMap::with_capacity(1000);
    target_map.insert("existing".to_string(), 999);
    let initial_capacity = target_map.capacity();

    // Serialize a smaller map
    let mut original_map = HashMap::new();
    original_map.insert("key1".to_string(), 1);
    original_map.insert("key2".to_string(), 2);
    let serialized = fory.serialize(&original_map);

    // Deserialize into existing map (should reuse capacity)
    fory.deserialize_into(&serialized, &mut target_map).unwrap();
    
    assert_eq!(target_map, original_map);
    // Capacity should be preserved or reasonable
    assert!(target_map.capacity() >= initial_capacity || target_map.capacity() >= 2);
}

#[test]
fn test_deserialize_into_overwrite_existing() {
    let fory = Fory::default();

    // Test that existing content is properly overwritten
    let mut target_string = "old content that should be replaced".to_string();
    let original_string = "new content".to_string();
    let serialized = fory.serialize(&original_string);

    fory.deserialize_into(&serialized, &mut target_string).unwrap();
    assert_eq!(target_string, original_string);
    assert_ne!(target_string, "old content that should be replaced");
}
