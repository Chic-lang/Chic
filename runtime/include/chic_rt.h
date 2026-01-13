#pragma once

#include <stdbool.h>
#include <stdalign.h>
#include <stddef.h>
#include <stdint.h>

#if !defined(__SIZEOF_INT128__)
#error "chic_rt.h requires compiler support for __int128 / unsigned __int128."
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define CHIC_RT_STRING_INLINE_CAPACITY 32
#define CHIC_RT_VEC_INLINE_BYTES 64

typedef uint16_t chic_char;

typedef struct ChicStr {
    const uint8_t *ptr;
    size_t len;
} ChicStr;

typedef struct ChicString {
    uint8_t *ptr;
    size_t len;
    size_t cap;
    uint8_t inline_data[CHIC_RT_STRING_INLINE_CAPACITY];
} ChicString;

typedef struct ChicCharSpan {
    const chic_char *ptr;
    size_t len;
} ChicCharSpan;

typedef enum StringError {
    ChicString_Success = 0,
    ChicString_Utf8 = 1,
    ChicString_CapacityOverflow = 2,
    ChicString_AllocationFailed = 3,
    ChicString_InvalidPointer = 4,
    ChicString_OutOfBounds = 5,
} StringError;

ChicStr chic_rt_string_error_message(int32_t code);

uint8_t *chic_rt_string_get_ptr(const ChicString *value);
void chic_rt_string_set_ptr(ChicString *value, uint8_t *ptr);
size_t chic_rt_string_get_len(const ChicString *value);
void chic_rt_string_set_len(ChicString *value, size_t len);
size_t chic_rt_string_get_cap(const ChicString *value);
void chic_rt_string_set_cap(ChicString *value, size_t cap);
uint8_t *chic_rt_string_inline_ptr(ChicString *value);
size_t chic_rt_string_inline_capacity(void);

ChicString chic_rt_string_new(void);
ChicString chic_rt_string_with_capacity(size_t capacity);
ChicString chic_rt_string_from_slice(ChicStr slice);
ChicString chic_rt_string_from_char(chic_char value);
void chic_rt_string_drop(ChicString *target);
int32_t chic_rt_string_clone(ChicString *dest, const ChicString *src);
int32_t chic_rt_string_clone_slice(ChicString *dest, ChicStr slice);
int32_t chic_rt_string_reserve(ChicString *target, size_t additional);
int32_t chic_rt_string_push_slice(ChicString *target, ChicStr slice);
int32_t chic_rt_string_truncate(ChicString *target, size_t new_len);
ChicStr chic_rt_string_as_slice(const ChicString *source);
ChicCharSpan chic_rt_string_as_chars(const ChicString *source);
ChicCharSpan chic_rt_str_as_chars(ChicStr slice);
int32_t chic_rt_string_append_slice(
    ChicString *target,
    ChicStr slice,
    int32_t alignment,
    int32_t has_alignment);
int32_t chic_rt_string_append_bool(
    ChicString *target,
    bool value,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_char(
    ChicString *target,
    chic_char value,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_signed(
    ChicString *target,
    uint64_t low,
    uint64_t high,
    uint32_t bits,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_unsigned(
    ChicString *target,
    uint64_t low,
    uint64_t high,
    uint32_t bits,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_f32(
    ChicString *target,
    float value,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_f64(
    ChicString *target,
    double value,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_f16(
    ChicString *target,
    uint16_t bits,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);
int32_t chic_rt_string_append_f128(
    ChicString *target,
    unsigned __int128 bits,
    int32_t alignment,
    int32_t has_alignment,
    ChicStr format);

typedef enum CharError {
    ChicChar_Success = 0,
    ChicChar_InvalidScalar = 1,
    ChicChar_NullPointer = 2,
    ChicChar_ComplexMapping = 3,
} CharError;

int32_t chic_rt_char_is_scalar(chic_char value);
int32_t chic_rt_char_is_digit(chic_char value);
int32_t chic_rt_char_is_letter(chic_char value);
int32_t chic_rt_char_is_whitespace(chic_char value);
uint64_t chic_rt_char_to_upper(chic_char value);
uint64_t chic_rt_char_to_lower(chic_char value);
uint64_t chic_rt_char_from_codepoint(uint32_t value);
int32_t chic_rt_char_status(uint64_t value);
chic_char chic_rt_char_value(uint64_t value);

typedef struct ValueConstPtr {
    const uint8_t *ptr;
    size_t size;
    size_t align;
} ValueConstPtr;

typedef struct ValueMutPtr {
    uint8_t *ptr;
    size_t size;
    size_t align;
} ValueMutPtr;

typedef struct ChicAllocatorVTable {
    void *context;
    ValueMutPtr (*alloc)(void *context, size_t size, size_t align);
    ValueMutPtr (*alloc_zeroed)(void *context, size_t size, size_t align);
    ValueMutPtr (*realloc)(
        void *context,
        ValueMutPtr existing,
        size_t old_size,
        size_t new_size,
        size_t align);
    void (*free)(void *context, ValueMutPtr ptr);
} ChicAllocatorVTable;

ValueMutPtr chic_rt_alloc(size_t size, size_t align);
ValueMutPtr chic_rt_alloc_zeroed(size_t size, size_t align);
ValueMutPtr chic_rt_realloc(ValueMutPtr ptr, size_t old_size, size_t new_size, size_t align);
void chic_rt_free(ValueMutPtr ptr);
void chic_rt_allocator_install(ChicAllocatorVTable vtable);
void chic_rt_allocator_reset(void);

typedef struct RegionHandle {
    void *ptr;
} RegionHandle;

typedef struct ChicInlineBuffer {
    uint8_t bytes[CHIC_RT_VEC_INLINE_BYTES];
} ChicInlineBuffer;

typedef struct ChicVec {
    uint8_t *ptr;
    size_t len;
    size_t cap;
    size_t elem_size;
    size_t elem_align;
    uintptr_t drop_fn;
    RegionHandle region;
    bool uses_inline;
    uint8_t _pad[3];
    uint32_t inline_pad;
    ChicInlineBuffer inline_storage;
} ChicVec;

typedef struct ChicVecView {
    const uint8_t *data;
    size_t len;
    size_t elem_size;
    size_t elem_align;
} ChicVecView;

typedef struct ChicVecIter {
    const uint8_t *data;
    size_t index;
    size_t len;
    size_t elem_size;
    size_t elem_align;
} ChicVecIter;

typedef enum VecError {
    ChicVec_Success = 0,
    ChicVec_AllocationFailed = 1,
    ChicVec_InvalidPointer = 2,
    ChicVec_CapacityOverflow = 3,
    ChicVec_OutOfBounds = 4,
    ChicVec_LengthOverflow = 5,
    ChicVec_IterationComplete = 6,
} VecError;

typedef struct ChicHashSet {
    uint8_t *entries;
    uint8_t *states;
    uint8_t *hashes;
    size_t len;
    size_t cap;
    size_t tombstones;
    size_t elem_size;
    size_t elem_align;
    uintptr_t drop_fn;
    uintptr_t eq_fn;
} ChicHashSet;

typedef struct ChicHashSetIter {
    const uint8_t *entries;
    const uint8_t *states;
    size_t index;
    size_t cap;
    size_t elem_size;
    size_t elem_align;
} ChicHashSetIter;

typedef enum HashSetError {
    ChicHashSet_Success = 0,
    ChicHashSet_AllocationFailed = 1,
    ChicHashSet_InvalidPointer = 2,
    ChicHashSet_CapacityOverflow = 3,
    ChicHashSet_NotFound = 4,
    ChicHashSet_IterationComplete = 5,
} HashSetError;

ChicVec chic_rt_vec_new(size_t elem_size, size_t elem_align, uintptr_t drop_fn);
ChicVec chic_rt_vec_new_in_region(
    size_t elem_size,
    size_t elem_align,
    uintptr_t drop_fn,
    RegionHandle region);
ChicVec chic_rt_vec_with_capacity(
    size_t elem_size,
    size_t elem_align,
    size_t capacity,
    uintptr_t drop_fn);
ChicVec chic_rt_vec_with_capacity_in_region(
    size_t elem_size,
    size_t elem_align,
    size_t capacity,
    uintptr_t drop_fn,
    RegionHandle region);
void chic_rt_vec_drop(ChicVec *vec);
int32_t chic_rt_vec_clone(ChicVec *dest, const ChicVec *src);
int32_t chic_rt_vec_into_array(ChicVec *dest, ChicVec *src);
int32_t chic_rt_array_into_vec(ChicVec *dest, ChicVec *src);
int32_t chic_rt_vec_reserve(ChicVec *vec, size_t additional);
int32_t chic_rt_vec_shrink_to_fit(ChicVec *vec);
int32_t chic_rt_vec_push(ChicVec *vec, const ValueConstPtr *value);
int32_t chic_rt_vec_pop(ChicVec *vec, const ValueMutPtr *out);
int32_t chic_rt_vec_insert(ChicVec *vec, size_t index, const ValueConstPtr *value);
int32_t chic_rt_vec_remove(ChicVec *vec, size_t index, const ValueMutPtr *out);
int32_t chic_rt_vec_swap_remove(ChicVec *vec, size_t index, const ValueMutPtr *out);
int32_t chic_rt_vec_truncate(ChicVec *vec, size_t new_len);
int32_t chic_rt_vec_clear(ChicVec *vec);
int32_t chic_rt_vec_set_len(ChicVec *vec, size_t new_len);
int32_t chic_rt_vec_copy_to_array(ChicVec *dest, const ChicVec *src);
int32_t chic_rt_array_copy_to_vec(ChicVec *dest, const ChicVec *src);
int32_t chic_rt_vec_iter_next(ChicVecIter *iter, const ValueMutPtr *out);
ValueConstPtr chic_rt_vec_iter_next_ptr(ChicVecIter *iter);

size_t chic_rt_vec_len(const ChicVec *vec);
size_t chic_rt_vec_capacity(const ChicVec *vec);
int32_t chic_rt_vec_is_empty(const ChicVec *vec);
int32_t chic_rt_vec_view(const ChicVec *vec, ChicVecView *out);
ValueConstPtr chic_rt_vec_data(const ChicVec *vec);
ValueMutPtr chic_rt_vec_data_mut(ChicVec *vec);
ChicVecIter chic_rt_vec_iter(const ChicVec *vec);
size_t chic_rt_vec_inline_capacity(const ChicVec *vec);
ValueMutPtr chic_rt_vec_inline_ptr(ChicVec *vec);
void chic_rt_vec_mark_inline(ChicVec *vec, int32_t uses_inline);
int32_t chic_rt_vec_uses_inline(const ChicVec *vec);
ChicVecView chic_rt_array_view(const ChicVec *array);
ValueConstPtr chic_rt_array_data(const ChicVec *array);
size_t chic_rt_array_len(const ChicVec *array);
int32_t chic_rt_array_is_empty(const ChicVec *array);
ValueMutPtr chic_rt_array_ptr_at(const ChicVec *array, size_t index);
ValueMutPtr chic_rt_vec_ptr_at(const ChicVec *vec, size_t index);

ValueMutPtr chic_rt_vec_get_ptr(const ChicVec *vec);
void chic_rt_vec_set_ptr(ChicVec *vec, const ValueMutPtr *ptr);
void chic_rt_vec_set_cap(ChicVec *vec, size_t cap);
size_t chic_rt_vec_elem_size(const ChicVec *vec);
size_t chic_rt_vec_elem_align(const ChicVec *vec);
void chic_rt_vec_set_elem_size(ChicVec *vec, size_t size);
void chic_rt_vec_set_elem_align(ChicVec *vec, size_t align);
uintptr_t chic_rt_vec_get_drop(const ChicVec *vec);
void chic_rt_vec_set_drop(ChicVec *vec, uintptr_t drop_fn);

ChicHashSet chic_rt_hashset_new(
    size_t elem_size,
    size_t elem_align,
    uintptr_t drop_fn,
    uintptr_t eq_fn);
ChicHashSet chic_rt_hashset_with_capacity(
    size_t elem_size,
    size_t elem_align,
    size_t capacity,
    uintptr_t drop_fn,
    uintptr_t eq_fn);
void chic_rt_hashset_drop(ChicHashSet *set);
HashSetError chic_rt_hashset_clear(ChicHashSet *set);
HashSetError chic_rt_hashset_reserve(ChicHashSet *set, size_t additional);
HashSetError chic_rt_hashset_shrink_to(ChicHashSet *set, size_t min_capacity);
size_t chic_rt_hashset_len(const ChicHashSet *set);
size_t chic_rt_hashset_capacity(const ChicHashSet *set);
size_t chic_rt_hashset_tombstones(const ChicHashSet *set);
HashSetError chic_rt_hashset_insert(
    ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *value,
    int32_t *inserted);
HashSetError chic_rt_hashset_replace(
    ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *value,
    const ValueMutPtr *out,
    int32_t *replaced);
int32_t chic_rt_hashset_contains(
    const ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *key);
ValueConstPtr chic_rt_hashset_get_ptr(
    const ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *key);
HashSetError chic_rt_hashset_take(
    ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *key,
    const ValueMutPtr *out);
int32_t chic_rt_hashset_remove(
    ChicHashSet *set,
    uint64_t hash,
    const ValueConstPtr *key);
HashSetError chic_rt_hashset_take_at(
    ChicHashSet *set,
    size_t index,
    const ValueMutPtr *out);
uint8_t chic_rt_hashset_bucket_state(const ChicHashSet *set, size_t index);
uint64_t chic_rt_hashset_bucket_hash(const ChicHashSet *set, size_t index);
ChicHashSetIter chic_rt_hashset_iter(const ChicHashSet *set);
HashSetError chic_rt_hashset_iter_next(ChicHashSetIter *iter, const ValueMutPtr *out);
ValueConstPtr chic_rt_hashset_iter_next_ptr(ChicHashSetIter *iter);

typedef struct ChicHashMap {
    uint8_t *entries;
    uint8_t *states;
    uint8_t *hashes;
    size_t len;
    size_t cap;
    size_t tombstones;
    size_t key_size;
    size_t key_align;
    size_t value_size;
    size_t value_align;
    size_t entry_size;
    size_t value_offset;
    uintptr_t key_drop_fn;
    uintptr_t value_drop_fn;
    uintptr_t key_eq_fn;
} ChicHashMap;

typedef struct ChicHashMapIter {
    const uint8_t *entries;
    const uint8_t *states;
    size_t index;
    size_t cap;
    size_t entry_size;
    size_t key_size;
    size_t key_align;
    size_t value_size;
    size_t value_align;
    size_t value_offset;
} ChicHashMapIter;

typedef enum HashMapError {
    ChicHashMap_Success = 0,
    ChicHashMap_AllocationFailed = 1,
    ChicHashMap_InvalidPointer = 2,
    ChicHashMap_CapacityOverflow = 3,
    ChicHashMap_NotFound = 4,
    ChicHashMap_IterationComplete = 5,
} HashMapError;

ChicHashMap chic_rt_hashmap_new(
    size_t key_size,
    size_t key_align,
    size_t value_size,
    size_t value_align,
    uintptr_t key_drop_fn,
    uintptr_t value_drop_fn,
    uintptr_t key_eq_fn);
ChicHashMap chic_rt_hashmap_with_capacity(
    size_t key_size,
    size_t key_align,
    size_t value_size,
    size_t value_align,
    size_t capacity,
    uintptr_t key_drop_fn,
    uintptr_t value_drop_fn,
    uintptr_t key_eq_fn);
void chic_rt_hashmap_drop(ChicHashMap *map);
HashMapError chic_rt_hashmap_clear(ChicHashMap *map);
HashMapError chic_rt_hashmap_reserve(ChicHashMap *map, size_t additional);
HashMapError chic_rt_hashmap_shrink_to(ChicHashMap *map, size_t min_capacity);
size_t chic_rt_hashmap_len(const ChicHashMap *map);
size_t chic_rt_hashmap_capacity(const ChicHashMap *map);
HashMapError chic_rt_hashmap_insert(
    ChicHashMap *map,
    uint64_t hash,
    const ValueConstPtr *key,
    const ValueConstPtr *value,
    const ValueMutPtr *previous,
    int32_t *replaced);
int32_t chic_rt_hashmap_contains(
    const ChicHashMap *map,
    uint64_t hash,
    const ValueConstPtr *key);
ValueConstPtr chic_rt_hashmap_get_ptr(
    const ChicHashMap *map,
    uint64_t hash,
    const ValueConstPtr *key);
HashMapError chic_rt_hashmap_take(
    ChicHashMap *map,
    uint64_t hash,
    const ValueConstPtr *key,
    const ValueMutPtr *out);
int32_t chic_rt_hashmap_remove(
    ChicHashMap *map,
    uint64_t hash,
    const ValueConstPtr *key);
HashMapError chic_rt_hashmap_take_at(
    ChicHashMap *map,
    size_t index,
    const ValueMutPtr *key_out,
    const ValueMutPtr *value_out);
uint8_t chic_rt_hashmap_bucket_state(const ChicHashMap *map, size_t index);
uint64_t chic_rt_hashmap_bucket_hash(const ChicHashMap *map, size_t index);
ChicHashMapIter chic_rt_hashmap_iter(const ChicHashMap *map);
HashMapError chic_rt_hashmap_iter_next(
    ChicHashMapIter *iter,
    const ValueMutPtr *key_out,
    const ValueMutPtr *value_out);
ValueConstPtr chic_rt_hashmap_iter_next_ptr(ChicHashMapIter *iter);

uint64_t chic_rt_hash_invoke(uintptr_t func, const void *value);
int32_t chic_rt_eq_invoke(uintptr_t func, const void *left, const void *right);

typedef struct ChicRc {
    void *ptr;
} ChicRc;

typedef struct ChicWeakRc {
    void *ptr;
} ChicWeakRc;

typedef struct ChicArc {
    void *ptr;
} ChicArc;

typedef struct ChicWeak {
    void *ptr;
} ChicWeak;

typedef enum SharedError {
    ChicShared_Success = 0,
    ChicShared_InvalidPointer = -1,
    ChicShared_AllocationFailed = -2,
    ChicShared_Overflow = -3,
} SharedError;

uint8_t *chic_rt_object_new(uint64_t type_id);

int32_t chic_rt_arc_new(
    ChicArc *dest,
    const uint8_t *src,
    size_t size,
    size_t align,
    uintptr_t drop_fn,
    uint64_t type_id);
int32_t chic_rt_arc_clone(ChicArc *dest, const ChicArc *src);
void chic_rt_arc_drop(ChicArc *target);
const uint8_t *chic_rt_arc_get(const ChicArc *src);
uint8_t *chic_rt_arc_get_mut(ChicArc *src);
uint8_t *chic_rt_arc_get_data(const ChicArc *handle);
size_t chic_rt_arc_strong_count(const ChicArc *src);
size_t chic_rt_arc_weak_count(const ChicArc *src);
int32_t chic_rt_arc_downgrade(ChicWeak *dest, const ChicArc *src);
int32_t chic_rt_weak_clone(ChicWeak *dest, const ChicWeak *src);
void chic_rt_weak_drop(ChicWeak *target);
int32_t chic_rt_weak_upgrade(ChicArc *dest, const ChicWeak *src);

int32_t chic_rt_rc_new(
    ChicRc *dest,
    const uint8_t *src,
    size_t size,
    size_t align,
    uintptr_t drop_fn,
    uint64_t type_id);
int32_t chic_rt_rc_clone(ChicRc *dest, const ChicRc *src);
void chic_rt_rc_drop(ChicRc *target);
const uint8_t *chic_rt_rc_get(const ChicRc *src);
uint8_t *chic_rt_rc_get_mut(ChicRc *src);
size_t chic_rt_rc_strong_count(const ChicRc *src);
size_t chic_rt_rc_weak_count(const ChicRc *src);
int32_t chic_rt_rc_downgrade(ChicWeakRc *dest, const ChicRc *src);
int32_t chic_rt_weak_rc_clone(ChicWeakRc *dest, const ChicWeakRc *src);
void chic_rt_weak_rc_drop(ChicWeakRc *target);
int32_t chic_rt_weak_rc_upgrade(ChicRc *dest, const ChicWeakRc *src);

#ifdef __cplusplus
}
#endif
