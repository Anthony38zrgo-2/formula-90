#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>
#include <godot_cpp/classes/ref_counted.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/string.hpp>

namespace godot {

class PhysicalVehicleWorldInterface : public RefCounted {
    GDCLASS(PhysicalVehicleWorldInterface, RefCounted)

    using InterfaceVersionFunction = uint32_t (*)();
    using BuildSourceFunction = const char *(*)();
    using ExecuteRequestFunction = char *(*)(const uint8_t *, size_t);
    using FreeResponseFunction = void (*)(char *);
    void *library_handle = nullptr;
    InterfaceVersionFunction interface_version_function = nullptr;
    BuildSourceFunction build_source_function = nullptr;
    ExecuteRequestFunction execute_request_function = nullptr;
    FreeResponseFunction free_response_function = nullptr;
    std::vector<uint64_t> owned_world_identifiers;

protected:
    static void _bind_methods();

public:
    ~PhysicalVehicleWorldInterface();
    bool initialize_library(const String &library_path);
    Dictionary execute_request(const Dictionary &request);
    void close_library();
};

}
