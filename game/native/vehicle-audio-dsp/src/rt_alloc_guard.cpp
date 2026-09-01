#include "dsp_instance.h"

#include <cstdlib>
#include <new>

void* operator new(std::size_t n) {
    if (n == 0) {
        n = 1;
    }
    void* p = std::malloc(n);
    if (!p) {
        throw std::bad_alloc();
    }
    f90dsp::rt_alloc_mark_alloc();
    return p;
}

void* operator new[](std::size_t n) { return ::operator new(n); }

void operator delete(void* p) noexcept {
    if (p) {
        f90dsp::rt_alloc_mark_free();
        std::free(p);
    }
}

void operator delete[](void* p) noexcept { ::operator delete(p); }

void operator delete(void* p, std::size_t) noexcept { ::operator delete(p); }

void operator delete[](void* p, std::size_t) noexcept { ::operator delete(p); }
