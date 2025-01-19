# Sparky Language


## Type

Integer types, arrays, pointers, function pointers, types, unit, tuple, struct, enum


```sprk

const fun hashmap_imp(key: type, value: type): type {
    assert(std.cmp.totalord.has_impl(key))
    mut kind = struct {
        buf = *(key, value),
        cap = usize,
        len = usize,
    }

    kind.new = fun(): kind {
        #kind {
            buf = str.ptr.null,
            cap = 0,
            len = 0,
        }
    }

    kind
}


const impls = hashmap_imp((type, type), type).new()

pub const fun hashmap(key: type, value: type): type {
    const impl = || hashmap_imp(key, value)
    impls.get(&(key, value)).or_else(|| hashmap_imp(key, value))
}

const fun mappedstruct(map: struct): type {
    
}


```
