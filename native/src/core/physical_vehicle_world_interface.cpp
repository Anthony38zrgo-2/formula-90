#include "formula90s/core/physical_vehicle_world_interface.hpp"
#include <godot_cpp/classes/json.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <algorithm>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#else
#include <dlfcn.h>
#endif

using namespace godot;

void PhysicalVehicleWorldInterface::_bind_methods() {
    ClassDB::bind_method(D_METHOD("initialize_library", "library_path"), &PhysicalVehicleWorldInterface::initialize_library);
    ClassDB::bind_method(D_METHOD("execute_request", "request"), &PhysicalVehicleWorldInterface::execute_request);
    ClassDB::bind_method(D_METHOD("close_library"), &PhysicalVehicleWorldInterface::close_library);
}

PhysicalVehicleWorldInterface::~PhysicalVehicleWorldInterface() { close_library(); }

bool PhysicalVehicleWorldInterface::initialize_library(const String &library_path) {
    close_library();
    if (library_handle) { return false; }
    const String absolute_path = ProjectSettings::get_singleton()->globalize_path(library_path);
#ifdef _WIN32
    library_handle = reinterpret_cast<void *>(LoadLibraryW(reinterpret_cast<const wchar_t *>(absolute_path.utf16().get_data())));
    if (!library_handle) { return false; }
    interface_version_function = reinterpret_cast<InterfaceVersionFunction>(GetProcAddress(reinterpret_cast<HMODULE>(library_handle), "coupled_vehicle_world_interface_version"));
    execute_request_function = reinterpret_cast<ExecuteRequestFunction>(GetProcAddress(reinterpret_cast<HMODULE>(library_handle), "coupled_vehicle_world_execute_request"));
    free_response_function = reinterpret_cast<FreeResponseFunction>(GetProcAddress(reinterpret_cast<HMODULE>(library_handle), "coupled_vehicle_world_free_response"));
#else
    library_handle = dlopen(absolute_path.utf8().get_data(), RTLD_NOW | RTLD_LOCAL);
    if (!library_handle) { return false; }
    interface_version_function = reinterpret_cast<InterfaceVersionFunction>(dlsym(library_handle, "coupled_vehicle_world_interface_version"));
    execute_request_function = reinterpret_cast<ExecuteRequestFunction>(dlsym(library_handle, "coupled_vehicle_world_execute_request"));
    free_response_function = reinterpret_cast<FreeResponseFunction>(dlsym(library_handle, "coupled_vehicle_world_free_response"));
#endif
    if (!interface_version_function || !execute_request_function || !free_response_function || interface_version_function() != 1) {
        close_library();
        return false;
    }
    return true;
}

Dictionary PhysicalVehicleWorldInterface::execute_request(const Dictionary &request) {
    Dictionary failure;
    failure["success"] = false;
    failure["error"] = "Physical world native interface is unavailable";
    if (!execute_request_function || !free_response_function) { return failure; }
    const CharString document = JSON::stringify(request).utf8();
    char *response = execute_request_function(reinterpret_cast<const uint8_t *>(document.get_data()), static_cast<size_t>(document.length()));
    if (!response) {
        failure["error"] = "Physical world returned an empty response";
        return failure;
    }
    const String response_document = String::utf8(response);
    free_response_function(response);
    const Variant parsed = JSON::parse_string(response_document);
    if (parsed.get_type() != Variant::DICTIONARY) {
        failure["error"] = "Physical world returned an invalid response document";
        return failure;
    }
    const Dictionary result = parsed;
    if (static_cast<int64_t>(result.get("interface_version", 0)) != 1) {
        failure["error"] = "Physical world response interface version mismatch";
        return failure;
    }
    if (static_cast<bool>(result.get("success", false))) {
        const String operation = request.get("operation", String());
        if (operation == "create_world") {
            const Dictionary payload = result.get("result", Dictionary());
            owned_world_identifiers.push_back(static_cast<uint64_t>(static_cast<int64_t>(payload.get("world_identifier", 0))));
        } else if (operation == "destroy_world") {
            const uint64_t identifier = static_cast<uint64_t>(static_cast<int64_t>(request.get("world_identifier", 0)));
            owned_world_identifiers.erase(std::remove(owned_world_identifiers.begin(), owned_world_identifiers.end(), identifier), owned_world_identifiers.end());
        }
    }
    return result;
}

void PhysicalVehicleWorldInterface::close_library() {
    const auto worlds_to_destroy = owned_world_identifiers;
    for (const uint64_t identifier : worlds_to_destroy) {
        Dictionary request;
        request["operation"] = "destroy_world";
        request["world_identifier"] = static_cast<int64_t>(identifier);
        const Dictionary response = execute_request(request);
        if (!static_cast<bool>(response.get("success", false))) {
            UtilityFunctions::push_error("Physical-world library cannot close while an owned world remains active");
            return;
        }
    }
    interface_version_function = nullptr;
    execute_request_function = nullptr;
    free_response_function = nullptr;
    if (!library_handle) { return; }
#ifdef _WIN32
    FreeLibrary(reinterpret_cast<HMODULE>(library_handle));
#else
    dlclose(library_handle);
#endif
    library_handle = nullptr;
}
