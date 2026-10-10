extern "C" {
    fn __tolower(_: __darwin_ct_rune_t) -> __darwin_ct_rune_t;
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcat(
        __s1: *mut ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn sprintf(
        _: *mut ::core::ffi::c_char,
        _: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn fabs(_: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn cJSON_Delete(item: *mut cJSON);
    fn cJSON_GetObjectItem(object: *const cJSON, string: *const ::core::ffi::c_char) -> *mut cJSON;
    fn cJSON_GetObjectItemCaseSensitive(
        object: *const cJSON,
        string: *const ::core::ffi::c_char,
    ) -> *mut cJSON;
    fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsString(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool;
    fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool;
    fn cJSON_CreateNull() -> *mut cJSON;
    fn cJSON_CreateString(string: *const ::core::ffi::c_char) -> *mut cJSON;
    fn cJSON_CreateArray() -> *mut cJSON;
    fn cJSON_CreateObject() -> *mut cJSON;
    fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemToObject(
        object: *mut cJSON,
        string: *const ::core::ffi::c_char,
        item: *mut cJSON,
    ) -> cJSON_bool;
    fn cJSON_DetachItemFromObject(
        object: *mut cJSON,
        string: *const ::core::ffi::c_char,
    ) -> *mut cJSON;
    fn cJSON_DetachItemFromObjectCaseSensitive(
        object: *mut cJSON,
        string: *const ::core::ffi::c_char,
    ) -> *mut cJSON;
    fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const ::core::ffi::c_char);
    fn cJSON_DeleteItemFromObjectCaseSensitive(
        object: *mut cJSON,
        string: *const ::core::ffi::c_char,
    );
    fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON;
    fn cJSON_malloc(size: size_t) -> *mut ::core::ffi::c_void;
    fn cJSON_free(object: *mut ::core::ffi::c_void);
}
pub type __darwin_ct_rune_t = ::core::ffi::c_int;
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_0: ::core::ffi::c_int,
    pub valuestring: *mut ::core::ffi::c_char,
    pub valueint: ::core::ffi::c_int,
    pub valuedouble: ::core::ffi::c_double,
    pub string: *mut ::core::ffi::c_char,
}
pub type cJSON_bool = ::core::ffi::c_int;
pub const COPY: patch_operation = 5;
pub type patch_operation = ::core::ffi::c_uint;
pub const TEST: patch_operation = 6;
pub const MOVE: patch_operation = 4;
pub const REPLACE: patch_operation = 3;
pub const REMOVE: patch_operation = 2;
pub const ADD: patch_operation = 1;
pub const INVALID: patch_operation = 0;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[inline]
pub unsafe extern "C" fn tolower(mut _c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return __tolower(_c as __darwin_ct_rune_t) as ::core::ffi::c_int;
}
pub const ULONG_MAX: ::core::ffi::c_ulong = 0xffffffffffffffff as ::core::ffi::c_ulong;
pub const DBL_EPSILON: ::core::ffi::c_double = __DBL_EPSILON__;
pub const cJSON_Invalid: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const cJSON_Number: ::core::ffi::c_int = 8;
pub const cJSON_String: ::core::ffi::c_int = 16;
pub const cJSON_Array: ::core::ffi::c_int = 32;
pub const cJSON_Object: ::core::ffi::c_int = 64;
pub const true_0: cJSON_bool = 1 as ::core::ffi::c_int;
pub const false_0: cJSON_bool = 0 as ::core::ffi::c_int;
unsafe extern "C" fn cJSONUtils_strdup(
    string: *const ::core::ffi::c_uchar,
) -> *mut ::core::ffi::c_uchar {
    let mut length: size_t = 0 as size_t;
    let mut copy: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    length = strlen(string as *const ::core::ffi::c_char)
        .wrapping_add(::core::mem::size_of::<[::core::ffi::c_char; 1]>() as size_t);
    copy = cJSON_malloc(length) as *mut ::core::ffi::c_uchar;
    if copy.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    }
    memcpy(
        copy as *mut ::core::ffi::c_void,
        string as *const ::core::ffi::c_void,
        length,
    );
    return copy;
}
unsafe extern "C" fn compare_strings(
    mut string1: *const ::core::ffi::c_uchar,
    mut string2: *const ::core::ffi::c_uchar,
    case_sensitive: cJSON_bool,
) -> ::core::ffi::c_int {
    if string1.is_null() || string2.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    if string1 == string2 {
        return 0 as ::core::ffi::c_int;
    }
    if case_sensitive != 0 {
        return strcmp(
            string1 as *const ::core::ffi::c_char,
            string2 as *const ::core::ffi::c_char,
        );
    }
    while tolower(*string1 as ::core::ffi::c_int) == tolower(*string2 as ::core::ffi::c_int) {
        if *string1 as ::core::ffi::c_int == '\0' as i32 {
            return 0 as ::core::ffi::c_int;
        }
        string1 = string1.offset(1);
        string2 = string2.offset(1);
    }
    return tolower(*string1 as ::core::ffi::c_int) - tolower(*string2 as ::core::ffi::c_int);
}
unsafe extern "C" fn compare_double(
    mut a: ::core::ffi::c_double,
    mut b: ::core::ffi::c_double,
) -> cJSON_bool {
    let mut maxVal: ::core::ffi::c_double = if fabs(a) > fabs(b) { fabs(a) } else { fabs(b) };
    return (fabs(a - b) <= maxVal * DBL_EPSILON) as ::core::ffi::c_int;
}
unsafe extern "C" fn compare_pointers(
    mut name: *const ::core::ffi::c_uchar,
    mut pointer: *const ::core::ffi::c_uchar,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if name.is_null() || pointer.is_null() {
        return false_0;
    }
    while *name as ::core::ffi::c_int != '\0' as i32
        && *pointer as ::core::ffi::c_int != '\0' as i32
        && *pointer as ::core::ffi::c_int != '/' as i32
    {
        if *pointer as ::core::ffi::c_int == '~' as i32 {
            if (*pointer.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != '0' as i32
                || *name as ::core::ffi::c_int != '~' as i32)
                && (*pointer.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    != '1' as i32
                    || *name as ::core::ffi::c_int != '/' as i32)
            {
                return false_0;
            } else {
                pointer = pointer.offset(1);
            }
        } else if case_sensitive == 0
            && tolower(*name as ::core::ffi::c_int) != tolower(*pointer as ::core::ffi::c_int)
            || case_sensitive != 0 && *name as ::core::ffi::c_int != *pointer as ::core::ffi::c_int
        {
            return false_0;
        }
        name = name.offset(1);
        pointer = pointer.offset(1);
    }
    if (*pointer as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        && *pointer as ::core::ffi::c_int != '/' as i32) as ::core::ffi::c_int
        != (*name as ::core::ffi::c_int != 0 as ::core::ffi::c_int) as ::core::ffi::c_int
    {
        return false_0;
    }
    return true_0;
}
unsafe extern "C" fn pointer_encoded_length(mut string: *const ::core::ffi::c_uchar) -> size_t {
    let mut length: size_t = 0;
    length = 0 as size_t;
    while *string as ::core::ffi::c_int != '\0' as i32 {
        if *string as ::core::ffi::c_int == '~' as i32
            || *string as ::core::ffi::c_int == '/' as i32
        {
            length = length.wrapping_add(1);
        }
        string = string.offset(1);
        length = length.wrapping_add(1);
    }
    return length;
}
unsafe extern "C" fn encode_string_as_pointer(
    mut destination: *mut ::core::ffi::c_uchar,
    mut source: *const ::core::ffi::c_uchar,
) {
    while *source.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '\0' as i32 {
        if *source.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32 {
            *destination.offset(0 as ::core::ffi::c_int as isize) =
                '~' as i32 as ::core::ffi::c_uchar;
            *destination.offset(1 as ::core::ffi::c_int as isize) =
                '1' as i32 as ::core::ffi::c_uchar;
            destination = destination.offset(1);
        } else if *source.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == '~' as i32
        {
            *destination.offset(0 as ::core::ffi::c_int as isize) =
                '~' as i32 as ::core::ffi::c_uchar;
            *destination.offset(1 as ::core::ffi::c_int as isize) =
                '0' as i32 as ::core::ffi::c_uchar;
            destination = destination.offset(1);
        } else {
            *destination.offset(0 as ::core::ffi::c_int as isize) =
                *source.offset(0 as ::core::ffi::c_int as isize);
        }
        source = source.offset(1);
        destination = destination.offset(1);
    }
    *destination.offset(0 as ::core::ffi::c_int as isize) = '\0' as i32 as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_FindPointerFromObjectTo(
    object: *const cJSON,
    target: *const cJSON,
) -> *mut ::core::ffi::c_char {
    let mut child_index: size_t = 0 as size_t;
    let mut current_child: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if object.is_null() || target.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    if object == target {
        return cJSONUtils_strdup(
            b"\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar,
        ) as *mut ::core::ffi::c_char;
    }
    current_child = (*object).child as *mut cJSON;
    while !current_child.is_null() {
        let mut target_pointer: *mut ::core::ffi::c_uchar =
            cJSONUtils_FindPointerFromObjectTo(current_child, target) as *mut ::core::ffi::c_uchar;
        if !target_pointer.is_null() {
            if cJSON_IsArray(object) != 0 {
                let mut full_pointer: *mut ::core::ffi::c_uchar = cJSON_malloc(
                    strlen(target_pointer as *mut ::core::ffi::c_char)
                        .wrapping_add(20 as size_t)
                        .wrapping_add(::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t),
                )
                    as *mut ::core::ffi::c_uchar;
                if child_index > ULONG_MAX as size_t {
                    cJSON_free(target_pointer as *mut ::core::ffi::c_void);
                    cJSON_free(full_pointer as *mut ::core::ffi::c_void);
                    return ::core::ptr::null_mut::<::core::ffi::c_char>();
                }
                sprintf(
                    full_pointer as *mut ::core::ffi::c_char,
                    b"/%lu%s\0" as *const u8 as *const ::core::ffi::c_char,
                    child_index as ::core::ffi::c_ulong,
                    target_pointer,
                );
                cJSON_free(target_pointer as *mut ::core::ffi::c_void);
                return full_pointer as *mut ::core::ffi::c_char;
            }
            if cJSON_IsObject(object) != 0 {
                let mut full_pointer_0: *mut ::core::ffi::c_uchar = cJSON_malloc(
                    strlen(target_pointer as *mut ::core::ffi::c_char)
                        .wrapping_add(pointer_encoded_length(
                            (*current_child).string as *mut ::core::ffi::c_uchar,
                        ))
                        .wrapping_add(2 as size_t),
                )
                    as *mut ::core::ffi::c_uchar;
                *full_pointer_0.offset(0 as ::core::ffi::c_int as isize) =
                    '/' as i32 as ::core::ffi::c_uchar;
                encode_string_as_pointer(
                    full_pointer_0.offset(1 as ::core::ffi::c_int as isize),
                    (*current_child).string as *mut ::core::ffi::c_uchar,
                );
                strcat(
                    full_pointer_0 as *mut ::core::ffi::c_char,
                    target_pointer as *mut ::core::ffi::c_char,
                );
                cJSON_free(target_pointer as *mut ::core::ffi::c_void);
                return full_pointer_0 as *mut ::core::ffi::c_char;
            }
            cJSON_free(target_pointer as *mut ::core::ffi::c_void);
            return ::core::ptr::null_mut::<::core::ffi::c_char>();
        }
        current_child = (*current_child).next as *mut cJSON;
        child_index = child_index.wrapping_add(1);
    }
    return ::core::ptr::null_mut::<::core::ffi::c_char>();
}
unsafe extern "C" fn get_array_item(mut array: *const cJSON, mut item: size_t) -> *mut cJSON {
    let mut child: *mut cJSON = if !array.is_null() {
        (*array).child as *mut cJSON
    } else {
        ::core::ptr::null_mut::<cJSON>()
    };
    while !child.is_null() && item > 0 as size_t {
        item = item.wrapping_sub(1);
        child = (*child).next as *mut cJSON;
    }
    return child;
}
unsafe extern "C" fn decode_array_index_from_pointer(
    pointer: *const ::core::ffi::c_uchar,
    index: *mut size_t,
) -> cJSON_bool {
    let mut parsed_index: size_t = 0 as size_t;
    let mut position: size_t = 0 as size_t;
    if *pointer.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '0' as i32
        && (*pointer.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '\0' as i32
            && *pointer.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != '/' as i32)
    {
        return 0 as cJSON_bool;
    }
    position = 0 as size_t;
    while *pointer.offset(position as isize) as ::core::ffi::c_int >= '0' as i32
        && *pointer.offset(position as isize) as ::core::ffi::c_int <= '9' as i32
    {
        parsed_index = (10 as size_t).wrapping_mul(parsed_index).wrapping_add(
            (*pointer.offset(position as isize) as ::core::ffi::c_int - '0' as i32) as size_t,
        );
        position = position.wrapping_add(1);
    }
    if *pointer.offset(position as isize) as ::core::ffi::c_int != '\0' as i32
        && *pointer.offset(position as isize) as ::core::ffi::c_int != '/' as i32
    {
        return 0 as cJSON_bool;
    }
    *index = parsed_index;
    return 1 as cJSON_bool;
}
unsafe extern "C" fn get_item_from_pointer(
    object: *mut cJSON,
    mut pointer: *const ::core::ffi::c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut current_element: *mut cJSON = object;
    if pointer.is_null() {
        return ::core::ptr::null_mut::<cJSON>();
    }
    while *pointer.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '/' as i32
        && !current_element.is_null()
    {
        pointer = pointer.offset(1);
        if cJSON_IsArray(current_element) != 0 {
            let mut index: size_t = 0 as size_t;
            if decode_array_index_from_pointer(
                pointer as *const ::core::ffi::c_uchar,
                &raw mut index,
            ) == 0
            {
                return ::core::ptr::null_mut::<cJSON>();
            }
            current_element = get_array_item(current_element, index);
        } else if cJSON_IsObject(current_element) != 0 {
            current_element = (*current_element).child as *mut cJSON;
            while !current_element.is_null()
                && compare_pointers(
                    (*current_element).string as *mut ::core::ffi::c_uchar,
                    pointer as *const ::core::ffi::c_uchar,
                    case_sensitive,
                ) == 0
            {
                current_element = (*current_element).next as *mut cJSON;
            }
        } else {
            return ::core::ptr::null_mut::<cJSON>();
        }
        while *pointer.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '\0' as i32
            && *pointer.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int != '/' as i32
        {
            pointer = pointer.offset(1);
        }
    }
    return current_element;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointer(
    object: *mut cJSON,
    mut pointer: *const ::core::ffi::c_char,
) -> *mut cJSON {
    return get_item_from_pointer(object, pointer, false_0);
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GetPointerCaseSensitive(
    object: *mut cJSON,
    mut pointer: *const ::core::ffi::c_char,
) -> *mut cJSON {
    return get_item_from_pointer(object, pointer, true_0);
}
unsafe extern "C" fn decode_pointer_inplace(mut string: *mut ::core::ffi::c_uchar) {
    let mut decoded_string: *mut ::core::ffi::c_uchar = string;
    if string.is_null() {
        return;
    }
    while *string != 0 {
        if *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '~' as i32 {
            if *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '0' as i32
            {
                *decoded_string.offset(0 as ::core::ffi::c_int as isize) =
                    '~' as i32 as ::core::ffi::c_uchar;
            } else if *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                == '1' as i32
            {
                *decoded_string.offset(1 as ::core::ffi::c_int as isize) =
                    '/' as i32 as ::core::ffi::c_uchar;
            } else {
                return;
            }
            string = string.offset(1);
        }
        decoded_string = decoded_string.offset(1);
        string = string.offset(1);
    }
    *decoded_string.offset(0 as ::core::ffi::c_int as isize) = '\0' as i32 as ::core::ffi::c_uchar;
}
unsafe extern "C" fn detach_item_from_array(
    mut array: *mut cJSON,
    mut which: size_t,
) -> *mut cJSON {
    let mut c: *mut cJSON = (*array).child as *mut cJSON;
    while !c.is_null() && which > 0 as size_t {
        c = (*c).next as *mut cJSON;
        which = which.wrapping_sub(1);
    }
    if c.is_null() {
        return ::core::ptr::null_mut::<cJSON>();
    }
    if c != (*array).child {
        (*(*c).prev).next = (*c).next;
    }
    if !(*c).next.is_null() {
        (*(*c).next).prev = (*c).prev;
    }
    if c == (*array).child {
        (*array).child = (*c).next;
    } else if (*c).next.is_null() {
        (*(*array).child).prev = (*c).prev;
    }
    (*c).next = ::core::ptr::null_mut::<cJSON>();
    (*c).prev = (*c).next;
    return c;
}
unsafe extern "C" fn detach_path(
    mut object: *mut cJSON,
    mut path: *const ::core::ffi::c_uchar,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut parent_pointer: *mut ::core::ffi::c_uchar =
        ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut child_pointer: *mut ::core::ffi::c_uchar =
        ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut parent: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut detached_item: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    parent_pointer = cJSONUtils_strdup(path);
    if !parent_pointer.is_null() {
        child_pointer = strrchr(parent_pointer as *mut ::core::ffi::c_char, '/' as i32)
            as *mut ::core::ffi::c_uchar;
        if !child_pointer.is_null() {
            *child_pointer.offset(0 as ::core::ffi::c_int as isize) =
                '\0' as i32 as ::core::ffi::c_uchar;
            child_pointer = child_pointer.offset(1);
            parent = get_item_from_pointer(
                object,
                parent_pointer as *mut ::core::ffi::c_char,
                case_sensitive,
            );
            decode_pointer_inplace(child_pointer);
            if cJSON_IsArray(parent) != 0 {
                let mut index: size_t = 0 as size_t;
                if !(decode_array_index_from_pointer(child_pointer, &raw mut index) == 0) {
                    detached_item = detach_item_from_array(parent, index);
                }
            } else if cJSON_IsObject(parent) != 0 {
                detached_item =
                    cJSON_DetachItemFromObject(parent, child_pointer as *mut ::core::ffi::c_char);
            }
        }
    }
    if !parent_pointer.is_null() {
        cJSON_free(parent_pointer as *mut ::core::ffi::c_void);
    }
    return detached_item;
}
unsafe extern "C" fn sort_list(mut list: *mut cJSON, case_sensitive: cJSON_bool) -> *mut cJSON {
    let mut first: *mut cJSON = list;
    let mut second: *mut cJSON = list;
    let mut current_item: *mut cJSON = list;
    let mut result: *mut cJSON = list;
    let mut result_tail: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if list.is_null() || (*list).next.is_null() {
        return result;
    }
    while !current_item.is_null()
        && !(*current_item).next.is_null()
        && compare_strings(
            (*current_item).string as *mut ::core::ffi::c_uchar,
            (*(*current_item).next).string as *mut ::core::ffi::c_uchar,
            case_sensitive,
        ) < 0 as ::core::ffi::c_int
    {
        current_item = (*current_item).next as *mut cJSON;
    }
    if current_item.is_null() || (*current_item).next.is_null() {
        return result;
    }
    current_item = list;
    while !current_item.is_null() {
        second = (*second).next as *mut cJSON;
        current_item = (*current_item).next as *mut cJSON;
        if !current_item.is_null() {
            current_item = (*current_item).next as *mut cJSON;
        }
    }
    if !second.is_null() && !(*second).prev.is_null() {
        (*(*second).prev).next = ::core::ptr::null_mut::<cJSON>();
        (*second).prev = ::core::ptr::null_mut::<cJSON>();
    }
    first = sort_list(first, case_sensitive);
    second = sort_list(second, case_sensitive);
    result = ::core::ptr::null_mut::<cJSON>();
    while !first.is_null() && !second.is_null() {
        let mut smaller: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
        if compare_strings(
            (*first).string as *mut ::core::ffi::c_uchar,
            (*second).string as *mut ::core::ffi::c_uchar,
            case_sensitive,
        ) < 0 as ::core::ffi::c_int
        {
            smaller = first;
        } else {
            smaller = second;
        }
        if result.is_null() {
            result_tail = smaller;
            result = smaller;
        } else {
            (*result_tail).next = smaller as *mut cJSON;
            (*smaller).prev = result_tail as *mut cJSON;
            result_tail = smaller;
        }
        if first == smaller {
            first = (*first).next as *mut cJSON;
        } else {
            second = (*second).next as *mut cJSON;
        }
    }
    if !first.is_null() {
        if result.is_null() {
            return first;
        }
        (*result_tail).next = first as *mut cJSON;
        (*first).prev = result_tail as *mut cJSON;
    }
    if !second.is_null() {
        if result.is_null() {
            return second;
        }
        (*result_tail).next = second as *mut cJSON;
        (*second).prev = result_tail as *mut cJSON;
    }
    return result;
}
unsafe extern "C" fn sort_object(object: *mut cJSON, case_sensitive: cJSON_bool) {
    if object.is_null() {
        return;
    }
    (*object).child = sort_list((*object).child as *mut cJSON, case_sensitive) as *mut cJSON;
}
unsafe extern "C" fn compare_json(
    mut a: *mut cJSON,
    mut b: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null()
        || b.is_null()
        || (*a).type_0 & 0xff as ::core::ffi::c_int != (*b).type_0 & 0xff as ::core::ffi::c_int
    {
        return false_0;
    }
    match (*a).type_0 & 0xff as ::core::ffi::c_int {
        cJSON_Number => {
            if (*a).valueint != (*b).valueint
                || compare_double((*a).valuedouble, (*b).valuedouble) == 0
            {
                return false_0;
            } else {
                return true_0;
            }
        }
        cJSON_String => {
            if strcmp((*a).valuestring, (*b).valuestring) != 0 as ::core::ffi::c_int {
                return false_0;
            } else {
                return true_0;
            }
        }
        cJSON_Array => {
            a = (*a).child as *mut cJSON;
            b = (*b).child as *mut cJSON;
            while !a.is_null() && !b.is_null() {
                let mut identical: cJSON_bool = compare_json(a, b, case_sensitive);
                if identical == 0 {
                    return false_0;
                }
                a = (*a).next as *mut cJSON;
                b = (*b).next as *mut cJSON;
            }
            if !a.is_null() || !b.is_null() {
                return false_0;
            } else {
                return true_0;
            }
        }
        cJSON_Object => {
            sort_object(a, case_sensitive);
            sort_object(b, case_sensitive);
            a = (*a).child as *mut cJSON;
            b = (*b).child as *mut cJSON;
            while !a.is_null() && !b.is_null() {
                let mut identical_0: cJSON_bool = false_0;
                if compare_strings(
                    (*a).string as *mut ::core::ffi::c_uchar,
                    (*b).string as *mut ::core::ffi::c_uchar,
                    case_sensitive,
                ) != 0
                {
                    return false_0;
                }
                identical_0 = compare_json(a, b, case_sensitive);
                if identical_0 == 0 {
                    return false_0;
                }
                a = (*a).next as *mut cJSON;
                b = (*b).next as *mut cJSON;
            }
            if !a.is_null() || !b.is_null() {
                return false_0;
            } else {
                return true_0;
            }
        }
        _ => {}
    }
    return true_0;
}
unsafe extern "C" fn insert_item_in_array(
    mut array: *mut cJSON,
    mut which: size_t,
    mut newitem: *mut cJSON,
) -> cJSON_bool {
    let mut child: *mut cJSON = (*array).child as *mut cJSON;
    while !child.is_null() && which > 0 as size_t {
        child = (*child).next as *mut cJSON;
        which = which.wrapping_sub(1);
    }
    if which > 0 as size_t {
        return 0 as cJSON_bool;
    }
    if child.is_null() {
        cJSON_AddItemToArray(array, newitem);
        return 1 as cJSON_bool;
    }
    (*newitem).next = child as *mut cJSON;
    (*newitem).prev = (*child).prev;
    (*child).prev = newitem as *mut cJSON;
    if child == (*array).child {
        (*array).child = newitem as *mut cJSON;
    } else {
        (*(*newitem).prev).next = newitem as *mut cJSON;
    }
    return 1 as cJSON_bool;
}
unsafe extern "C" fn get_object_item(
    object: *const cJSON,
    mut name: *const ::core::ffi::c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if case_sensitive != 0 {
        return cJSON_GetObjectItemCaseSensitive(object, name);
    }
    return cJSON_GetObjectItem(object, name);
}
unsafe extern "C" fn decode_patch_operation(
    patch: *const cJSON,
    case_sensitive: cJSON_bool,
) -> patch_operation {
    let mut operation: *mut cJSON = get_object_item(
        patch,
        b"op\0" as *const u8 as *const ::core::ffi::c_char,
        case_sensitive,
    );
    if cJSON_IsString(operation) == 0 {
        return INVALID;
    }
    if strcmp(
        (*operation).valuestring,
        b"add\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return ADD;
    }
    if strcmp(
        (*operation).valuestring,
        b"remove\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return REMOVE;
    }
    if strcmp(
        (*operation).valuestring,
        b"replace\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return REPLACE;
    }
    if strcmp(
        (*operation).valuestring,
        b"move\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return MOVE;
    }
    if strcmp(
        (*operation).valuestring,
        b"copy\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return COPY;
    }
    if strcmp(
        (*operation).valuestring,
        b"test\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return TEST;
    }
    return INVALID;
}
unsafe extern "C" fn overwrite_item(root: *mut cJSON, replacement: cJSON) {
    if root.is_null() {
        return;
    }
    if !(*root).string.is_null() {
        cJSON_free((*root).string as *mut ::core::ffi::c_void);
    }
    if !(*root).valuestring.is_null() {
        cJSON_free((*root).valuestring as *mut ::core::ffi::c_void);
    }
    if !(*root).child.is_null() {
        cJSON_Delete((*root).child as *mut cJSON);
    }
    memcpy(
        root as *mut ::core::ffi::c_void,
        &raw const replacement as *const ::core::ffi::c_void,
        ::core::mem::size_of::<cJSON>() as size_t,
    );
}
unsafe extern "C" fn apply_patch(
    mut object: *mut cJSON,
    mut patch: *const cJSON,
    case_sensitive: cJSON_bool,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut path: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut value: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut parent: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut opcode: patch_operation = INVALID;
    let mut parent_pointer: *mut ::core::ffi::c_uchar =
        ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut child_pointer: *mut ::core::ffi::c_uchar =
        ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    let mut status: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    path = get_object_item(
        patch,
        b"path\0" as *const u8 as *const ::core::ffi::c_char,
        case_sensitive,
    );
    if cJSON_IsString(path) == 0 {
        status = 2 as ::core::ffi::c_int;
    } else {
        opcode = decode_patch_operation(patch, case_sensitive);
        if opcode as ::core::ffi::c_uint == INVALID as ::core::ffi::c_int as ::core::ffi::c_uint {
            status = 3 as ::core::ffi::c_int;
        } else if opcode as ::core::ffi::c_uint == TEST as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            status = (compare_json(
                get_item_from_pointer(object, (*path).valuestring, case_sensitive),
                get_object_item(
                    patch,
                    b"value\0" as *const u8 as *const ::core::ffi::c_char,
                    case_sensitive,
                ),
                case_sensitive,
            ) == 0) as ::core::ffi::c_int;
        } else {
            if *(*path).valuestring.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                == '\0' as i32
            {
                if opcode as ::core::ffi::c_uint
                    == REMOVE as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    static mut invalid: cJSON = cJSON {
                        next: ::core::ptr::null::<cJSON>() as *mut cJSON,
                        prev: ::core::ptr::null::<cJSON>() as *mut cJSON,
                        child: ::core::ptr::null::<cJSON>() as *mut cJSON,
                        type_0: cJSON_Invalid,
                        valuestring: ::core::ptr::null::<::core::ffi::c_char>()
                            as *mut ::core::ffi::c_char,
                        valueint: 0 as ::core::ffi::c_int,
                        valuedouble: 0 as ::core::ffi::c_int as ::core::ffi::c_double,
                        string: ::core::ptr::null::<::core::ffi::c_char>()
                            as *mut ::core::ffi::c_char,
                    };
                    overwrite_item(object, invalid);
                    status = 0 as ::core::ffi::c_int;
                    current_block = 3186350029337015785;
                } else if opcode as ::core::ffi::c_uint
                    == REPLACE as ::core::ffi::c_int as ::core::ffi::c_uint
                    || opcode as ::core::ffi::c_uint
                        == ADD as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    value = get_object_item(
                        patch,
                        b"value\0" as *const u8 as *const ::core::ffi::c_char,
                        case_sensitive,
                    );
                    if value.is_null() {
                        status = 7 as ::core::ffi::c_int;
                    } else {
                        value = cJSON_Duplicate(value, 1 as cJSON_bool);
                        if value.is_null() {
                            status = 8 as ::core::ffi::c_int;
                        } else {
                            overwrite_item(object, *value);
                            cJSON_free(value as *mut ::core::ffi::c_void);
                            value = ::core::ptr::null_mut::<cJSON>();
                            if !(*object).string.is_null() {
                                cJSON_free((*object).string as *mut ::core::ffi::c_void);
                                (*object).string = ::core::ptr::null_mut::<::core::ffi::c_char>();
                            }
                            status = 0 as ::core::ffi::c_int;
                        }
                    }
                    current_block = 3186350029337015785;
                } else {
                    current_block = 4068382217303356765;
                }
            } else {
                current_block = 4068382217303356765;
            }
            match current_block {
                3186350029337015785 => {}
                _ => {
                    if opcode as ::core::ffi::c_uint
                        == REMOVE as ::core::ffi::c_int as ::core::ffi::c_uint
                        || opcode as ::core::ffi::c_uint
                            == REPLACE as ::core::ffi::c_int as ::core::ffi::c_uint
                    {
                        let mut old_item: *mut cJSON = detach_path(
                            object,
                            (*path).valuestring as *mut ::core::ffi::c_uchar,
                            case_sensitive,
                        );
                        if old_item.is_null() {
                            status = 13 as ::core::ffi::c_int;
                            current_block = 3186350029337015785;
                        } else {
                            cJSON_Delete(old_item);
                            if opcode as ::core::ffi::c_uint
                                == REMOVE as ::core::ffi::c_int as ::core::ffi::c_uint
                            {
                                status = 0 as ::core::ffi::c_int;
                                current_block = 3186350029337015785;
                            } else {
                                current_block = 1538046216550696469;
                            }
                        }
                    } else {
                        current_block = 1538046216550696469;
                    }
                    match current_block {
                        3186350029337015785 => {}
                        _ => {
                            if opcode as ::core::ffi::c_uint
                                == MOVE as ::core::ffi::c_int as ::core::ffi::c_uint
                                || opcode as ::core::ffi::c_uint
                                    == COPY as ::core::ffi::c_int as ::core::ffi::c_uint
                            {
                                let mut from: *mut cJSON = get_object_item(
                                    patch,
                                    b"from\0" as *const u8 as *const ::core::ffi::c_char,
                                    case_sensitive,
                                );
                                if cJSON_IsString(from) == 0 {
                                    status = 4 as ::core::ffi::c_int;
                                    current_block = 3186350029337015785;
                                } else {
                                    if opcode as ::core::ffi::c_uint
                                        == MOVE as ::core::ffi::c_int as ::core::ffi::c_uint
                                    {
                                        value = detach_path(
                                            object,
                                            (*from).valuestring as *mut ::core::ffi::c_uchar,
                                            case_sensitive,
                                        );
                                    }
                                    if opcode as ::core::ffi::c_uint
                                        == COPY as ::core::ffi::c_int as ::core::ffi::c_uint
                                    {
                                        value = get_item_from_pointer(
                                            object,
                                            (*from).valuestring,
                                            case_sensitive,
                                        );
                                    }
                                    if value.is_null() {
                                        status = 5 as ::core::ffi::c_int;
                                        current_block = 3186350029337015785;
                                    } else {
                                        if opcode as ::core::ffi::c_uint
                                            == COPY as ::core::ffi::c_int as ::core::ffi::c_uint
                                        {
                                            value = cJSON_Duplicate(value, 1 as cJSON_bool);
                                        }
                                        if value.is_null() {
                                            status = 6 as ::core::ffi::c_int;
                                            current_block = 3186350029337015785;
                                        } else {
                                            current_block = 1724319918354933278;
                                        }
                                    }
                                }
                            } else {
                                value = get_object_item(
                                    patch,
                                    b"value\0" as *const u8 as *const ::core::ffi::c_char,
                                    case_sensitive,
                                );
                                if value.is_null() {
                                    status = 7 as ::core::ffi::c_int;
                                    current_block = 3186350029337015785;
                                } else {
                                    value = cJSON_Duplicate(value, 1 as cJSON_bool);
                                    if value.is_null() {
                                        status = 8 as ::core::ffi::c_int;
                                        current_block = 3186350029337015785;
                                    } else {
                                        current_block = 1724319918354933278;
                                    }
                                }
                            }
                            match current_block {
                                3186350029337015785 => {}
                                _ => {
                                    parent_pointer = cJSONUtils_strdup(
                                        (*path).valuestring as *mut ::core::ffi::c_uchar,
                                    );
                                    if !parent_pointer.is_null() {
                                        child_pointer = strrchr(
                                            parent_pointer as *mut ::core::ffi::c_char,
                                            '/' as i32,
                                        )
                                            as *mut ::core::ffi::c_uchar;
                                    }
                                    if !child_pointer.is_null() {
                                        *child_pointer.offset(0 as ::core::ffi::c_int as isize) =
                                            '\0' as i32 as ::core::ffi::c_uchar;
                                        child_pointer = child_pointer.offset(1);
                                    }
                                    parent = get_item_from_pointer(
                                        object,
                                        parent_pointer as *mut ::core::ffi::c_char,
                                        case_sensitive,
                                    );
                                    decode_pointer_inplace(child_pointer);
                                    if parent.is_null() || child_pointer.is_null() {
                                        status = 9 as ::core::ffi::c_int;
                                    } else if cJSON_IsArray(parent) != 0 {
                                        if strcmp(
                                            child_pointer as *mut ::core::ffi::c_char,
                                            b"-\0" as *const u8 as *const ::core::ffi::c_char,
                                        ) == 0 as ::core::ffi::c_int
                                        {
                                            cJSON_AddItemToArray(parent, value);
                                            value = ::core::ptr::null_mut::<cJSON>();
                                        } else {
                                            let mut index: size_t = 0 as size_t;
                                            if decode_array_index_from_pointer(
                                                child_pointer,
                                                &raw mut index,
                                            ) == 0
                                            {
                                                status = 11 as ::core::ffi::c_int;
                                            } else if insert_item_in_array(parent, index, value)
                                                == 0
                                            {
                                                status = 10 as ::core::ffi::c_int;
                                            } else {
                                                value = ::core::ptr::null_mut::<cJSON>();
                                            }
                                        }
                                    } else if cJSON_IsObject(parent) != 0 {
                                        if case_sensitive != 0 {
                                            cJSON_DeleteItemFromObjectCaseSensitive(
                                                parent,
                                                child_pointer as *mut ::core::ffi::c_char,
                                            );
                                        } else {
                                            cJSON_DeleteItemFromObject(
                                                parent,
                                                child_pointer as *mut ::core::ffi::c_char,
                                            );
                                        }
                                        cJSON_AddItemToObject(
                                            parent,
                                            child_pointer as *mut ::core::ffi::c_char,
                                            value,
                                        );
                                        value = ::core::ptr::null_mut::<cJSON>();
                                    } else {
                                        status = 9 as ::core::ffi::c_int;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !value.is_null() {
        cJSON_Delete(value);
    }
    if !parent_pointer.is_null() {
        cJSON_free(parent_pointer as *mut ::core::ffi::c_void);
    }
    return status;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatches(
    object: *mut cJSON,
    patches: *const cJSON,
) -> ::core::ffi::c_int {
    let mut current_patch: *const cJSON = ::core::ptr::null::<cJSON>();
    let mut status: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if cJSON_IsArray(patches) == 0 {
        return 1 as ::core::ffi::c_int;
    }
    if !patches.is_null() {
        current_patch = (*patches).child;
    }
    while !current_patch.is_null() {
        status = apply_patch(object, current_patch, false_0);
        if status != 0 as ::core::ffi::c_int {
            return status;
        }
        current_patch = (*current_patch).next;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_ApplyPatchesCaseSensitive(
    object: *mut cJSON,
    patches: *const cJSON,
) -> ::core::ffi::c_int {
    let mut current_patch: *const cJSON = ::core::ptr::null::<cJSON>();
    let mut status: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if cJSON_IsArray(patches) == 0 {
        return 1 as ::core::ffi::c_int;
    }
    if !patches.is_null() {
        current_patch = (*patches).child;
    }
    while !current_patch.is_null() {
        status = apply_patch(object, current_patch, true_0);
        if status != 0 as ::core::ffi::c_int {
            return status;
        }
        current_patch = (*current_patch).next;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn compose_patch(
    patches: *mut cJSON,
    operation: *const ::core::ffi::c_uchar,
    path: *const ::core::ffi::c_uchar,
    mut suffix: *const ::core::ffi::c_uchar,
    value: *const cJSON,
) {
    let mut patch: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if patches.is_null() || operation.is_null() || path.is_null() {
        return;
    }
    patch = cJSON_CreateObject();
    if patch.is_null() {
        return;
    }
    cJSON_AddItemToObject(
        patch,
        b"op\0" as *const u8 as *const ::core::ffi::c_char,
        cJSON_CreateString(operation as *const ::core::ffi::c_char),
    );
    if suffix.is_null() {
        cJSON_AddItemToObject(
            patch,
            b"path\0" as *const u8 as *const ::core::ffi::c_char,
            cJSON_CreateString(path as *const ::core::ffi::c_char),
        );
    } else {
        let mut suffix_length: size_t = pointer_encoded_length(suffix);
        let mut path_length: size_t = strlen(path as *const ::core::ffi::c_char);
        let mut full_path: *mut ::core::ffi::c_uchar = cJSON_malloc(
            path_length
                .wrapping_add(suffix_length)
                .wrapping_add(::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t),
        ) as *mut ::core::ffi::c_uchar;
        sprintf(
            full_path as *mut ::core::ffi::c_char,
            b"%s/\0" as *const u8 as *const ::core::ffi::c_char,
            path as *const ::core::ffi::c_char,
        );
        encode_string_as_pointer(
            full_path
                .offset(path_length as isize)
                .offset(1 as ::core::ffi::c_int as isize),
            suffix,
        );
        cJSON_AddItemToObject(
            patch,
            b"path\0" as *const u8 as *const ::core::ffi::c_char,
            cJSON_CreateString(full_path as *const ::core::ffi::c_char),
        );
        cJSON_free(full_path as *mut ::core::ffi::c_void);
    }
    if !value.is_null() {
        cJSON_AddItemToObject(
            patch,
            b"value\0" as *const u8 as *const ::core::ffi::c_char,
            cJSON_Duplicate(value, 1 as cJSON_bool),
        );
    }
    cJSON_AddItemToArray(patches, patch);
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_AddPatchToArray(
    array: *mut cJSON,
    operation: *const ::core::ffi::c_char,
    path: *const ::core::ffi::c_char,
    value: *const cJSON,
) {
    compose_patch(
        array,
        operation as *const ::core::ffi::c_uchar,
        path as *const ::core::ffi::c_uchar,
        ::core::ptr::null::<::core::ffi::c_uchar>(),
        value,
    );
}
unsafe extern "C" fn create_patches(
    patches: *mut cJSON,
    path: *const ::core::ffi::c_uchar,
    from: *mut cJSON,
    to: *mut cJSON,
    case_sensitive: cJSON_bool,
) {
    if from.is_null() || to.is_null() {
        return;
    }
    if (*from).type_0 & 0xff as ::core::ffi::c_int != (*to).type_0 & 0xff as ::core::ffi::c_int {
        compose_patch(
            patches,
            b"replace\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar,
            path,
            ::core::ptr::null::<::core::ffi::c_uchar>(),
            to,
        );
        return;
    }
    match (*from).type_0 & 0xff as ::core::ffi::c_int {
        cJSON_Number => {
            if (*from).valueint != (*to).valueint
                || compare_double((*from).valuedouble, (*to).valuedouble) == 0
            {
                compose_patch(
                    patches,
                    b"replace\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_uchar,
                    path,
                    ::core::ptr::null::<::core::ffi::c_uchar>(),
                    to,
                );
            }
            return;
        }
        cJSON_String => {
            if strcmp((*from).valuestring, (*to).valuestring) != 0 as ::core::ffi::c_int {
                compose_patch(
                    patches,
                    b"replace\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_uchar,
                    path,
                    ::core::ptr::null::<::core::ffi::c_uchar>(),
                    to,
                );
            }
            return;
        }
        cJSON_Array => {
            let mut index: size_t = 0 as size_t;
            let mut from_child: *mut cJSON = (*from).child as *mut cJSON;
            let mut to_child: *mut cJSON = (*to).child as *mut cJSON;
            let mut new_path: *mut ::core::ffi::c_uchar = cJSON_malloc(
                strlen(path as *const ::core::ffi::c_char)
                    .wrapping_add(20 as size_t)
                    .wrapping_add(::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t),
            )
                as *mut ::core::ffi::c_uchar;
            index = 0 as size_t;
            while !from_child.is_null() && !to_child.is_null() {
                if index > ULONG_MAX as size_t {
                    cJSON_free(new_path as *mut ::core::ffi::c_void);
                    return;
                }
                sprintf(
                    new_path as *mut ::core::ffi::c_char,
                    b"%s/%lu\0" as *const u8 as *const ::core::ffi::c_char,
                    path,
                    index as ::core::ffi::c_ulong,
                );
                create_patches(patches, new_path, from_child, to_child, case_sensitive);
                from_child = (*from_child).next as *mut cJSON;
                to_child = (*to_child).next as *mut cJSON;
                index = index.wrapping_add(1);
            }
            while !from_child.is_null() {
                if index > ULONG_MAX as size_t {
                    cJSON_free(new_path as *mut ::core::ffi::c_void);
                    return;
                }
                sprintf(
                    new_path as *mut ::core::ffi::c_char,
                    b"%lu\0" as *const u8 as *const ::core::ffi::c_char,
                    index as ::core::ffi::c_ulong,
                );
                compose_patch(
                    patches,
                    b"remove\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_uchar,
                    path,
                    new_path,
                    ::core::ptr::null::<cJSON>(),
                );
                from_child = (*from_child).next as *mut cJSON;
            }
            while !to_child.is_null() {
                compose_patch(
                    patches,
                    b"add\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_uchar,
                    path,
                    b"-\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_uchar,
                    to_child,
                );
                to_child = (*to_child).next as *mut cJSON;
                index = index.wrapping_add(1);
            }
            cJSON_free(new_path as *mut ::core::ffi::c_void);
            return;
        }
        cJSON_Object => {
            let mut from_child_0: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
            let mut to_child_0: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
            sort_object(from, case_sensitive);
            sort_object(to, case_sensitive);
            from_child_0 = (*from).child as *mut cJSON;
            to_child_0 = (*to).child as *mut cJSON;
            while !from_child_0.is_null() || !to_child_0.is_null() {
                let mut diff: ::core::ffi::c_int = 0;
                if from_child_0.is_null() {
                    diff = 1 as ::core::ffi::c_int;
                } else if to_child_0.is_null() {
                    diff = -(1 as ::core::ffi::c_int);
                } else {
                    diff = compare_strings(
                        (*from_child_0).string as *mut ::core::ffi::c_uchar,
                        (*to_child_0).string as *mut ::core::ffi::c_uchar,
                        case_sensitive,
                    );
                }
                if diff == 0 as ::core::ffi::c_int {
                    let mut path_length: size_t = strlen(path as *const ::core::ffi::c_char);
                    let mut from_child_name_length: size_t =
                        pointer_encoded_length((*from_child_0).string as *mut ::core::ffi::c_uchar);
                    let mut new_path_0: *mut ::core::ffi::c_uchar = cJSON_malloc(
                        path_length
                            .wrapping_add(from_child_name_length)
                            .wrapping_add(
                                ::core::mem::size_of::<[::core::ffi::c_char; 2]>() as size_t
                            ),
                    )
                        as *mut ::core::ffi::c_uchar;
                    sprintf(
                        new_path_0 as *mut ::core::ffi::c_char,
                        b"%s/\0" as *const u8 as *const ::core::ffi::c_char,
                        path,
                    );
                    encode_string_as_pointer(
                        new_path_0
                            .offset(path_length as isize)
                            .offset(1 as ::core::ffi::c_int as isize),
                        (*from_child_0).string as *mut ::core::ffi::c_uchar,
                    );
                    create_patches(
                        patches,
                        new_path_0,
                        from_child_0,
                        to_child_0,
                        case_sensitive,
                    );
                    cJSON_free(new_path_0 as *mut ::core::ffi::c_void);
                    from_child_0 = (*from_child_0).next as *mut cJSON;
                    to_child_0 = (*to_child_0).next as *mut cJSON;
                } else if diff < 0 as ::core::ffi::c_int {
                    compose_patch(
                        patches,
                        b"remove\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_uchar,
                        path,
                        (*from_child_0).string as *mut ::core::ffi::c_uchar,
                        ::core::ptr::null::<cJSON>(),
                    );
                    from_child_0 = (*from_child_0).next as *mut cJSON;
                } else {
                    compose_patch(
                        patches,
                        b"add\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_uchar,
                        path,
                        (*to_child_0).string as *mut ::core::ffi::c_uchar,
                        to_child_0,
                    );
                    to_child_0 = (*to_child_0).next as *mut cJSON;
                }
            }
            return;
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatches(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    let mut patches: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if from.is_null() || to.is_null() {
        return ::core::ptr::null_mut::<cJSON>();
    }
    patches = cJSON_CreateArray();
    create_patches(
        patches,
        b"\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar,
        from,
        to,
        false_0,
    );
    return patches;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GeneratePatchesCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    let mut patches: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if from.is_null() || to.is_null() {
        return ::core::ptr::null_mut::<cJSON>();
    }
    patches = cJSON_CreateArray();
    create_patches(
        patches,
        b"\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar,
        from,
        to,
        true_0,
    );
    return patches;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObject(object: *mut cJSON) {
    sort_object(object, false_0);
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_SortObjectCaseSensitive(object: *mut cJSON) {
    sort_object(object, true_0);
}
unsafe extern "C" fn merge_patch(
    mut target: *mut cJSON,
    patch: *const cJSON,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut patch_child: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if cJSON_IsObject(patch) == 0 {
        let mut duplicate: *mut cJSON = cJSON_Duplicate(patch, 1 as cJSON_bool);
        cJSON_Delete(target);
        return duplicate;
    }
    if cJSON_IsObject(target) == 0 {
        cJSON_Delete(target);
        target = cJSON_CreateObject();
    }
    patch_child = (*patch).child as *mut cJSON;
    while !patch_child.is_null() {
        if cJSON_IsNull(patch_child) != 0 {
            if case_sensitive != 0 {
                cJSON_DeleteItemFromObjectCaseSensitive(target, (*patch_child).string);
            } else {
                cJSON_DeleteItemFromObject(target, (*patch_child).string);
            }
        } else {
            let mut replace_me: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
            let mut replacement: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
            if case_sensitive != 0 {
                replace_me = cJSON_DetachItemFromObjectCaseSensitive(target, (*patch_child).string);
            } else {
                replace_me = cJSON_DetachItemFromObject(target, (*patch_child).string);
            }
            replacement = merge_patch(replace_me, patch_child, case_sensitive);
            if replacement.is_null() {
                cJSON_Delete(target);
                return ::core::ptr::null_mut::<cJSON>();
            }
            cJSON_AddItemToObject(target, (*patch_child).string, replacement);
        }
        patch_child = (*patch_child).next as *mut cJSON;
    }
    return target;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatch(
    mut target: *mut cJSON,
    patch: *const cJSON,
) -> *mut cJSON {
    return merge_patch(target, patch, false_0);
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_MergePatchCaseSensitive(
    mut target: *mut cJSON,
    patch: *const cJSON,
) -> *mut cJSON {
    return merge_patch(target, patch, true_0);
}
unsafe extern "C" fn generate_merge_patch(
    from: *mut cJSON,
    to: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut from_child: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut to_child: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut patch: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    if to.is_null() {
        return cJSON_CreateNull();
    }
    if cJSON_IsObject(to) == 0 || cJSON_IsObject(from) == 0 {
        return cJSON_Duplicate(to, 1 as cJSON_bool);
    }
    sort_object(from, case_sensitive);
    sort_object(to, case_sensitive);
    from_child = (*from).child as *mut cJSON;
    to_child = (*to).child as *mut cJSON;
    patch = cJSON_CreateObject();
    if patch.is_null() {
        return ::core::ptr::null_mut::<cJSON>();
    }
    while !from_child.is_null() || !to_child.is_null() {
        let mut diff: ::core::ffi::c_int = 0;
        if !from_child.is_null() {
            if !to_child.is_null() {
                diff = strcmp((*from_child).string, (*to_child).string);
            } else {
                diff = -(1 as ::core::ffi::c_int);
            }
        } else {
            diff = 1 as ::core::ffi::c_int;
        }
        if diff < 0 as ::core::ffi::c_int {
            cJSON_AddItemToObject(patch, (*from_child).string, cJSON_CreateNull());
            from_child = (*from_child).next as *mut cJSON;
        } else if diff > 0 as ::core::ffi::c_int {
            cJSON_AddItemToObject(
                patch,
                (*to_child).string,
                cJSON_Duplicate(to_child, 1 as cJSON_bool),
            );
            to_child = (*to_child).next as *mut cJSON;
        } else {
            if compare_json(from_child, to_child, case_sensitive) == 0 {
                cJSON_AddItemToObject(
                    patch,
                    (*to_child).string,
                    cJSONUtils_GenerateMergePatch(from_child, to_child),
                );
            }
            from_child = (*from_child).next as *mut cJSON;
            to_child = (*to_child).next as *mut cJSON;
        }
    }
    if (*patch).child.is_null() {
        cJSON_Delete(patch);
        return ::core::ptr::null_mut::<cJSON>();
    }
    return patch;
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatch(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    return generate_merge_patch(from, to, false_0);
}
#[no_mangle]
pub unsafe extern "C" fn cJSONUtils_GenerateMergePatchCaseSensitive(
    from: *mut cJSON,
    to: *mut cJSON,
) -> *mut cJSON {
    return generate_merge_patch(from, to, true_0);
}
pub const __DBL_EPSILON__: ::core::ffi::c_double = 2.2204460492503131e-16f64;
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
