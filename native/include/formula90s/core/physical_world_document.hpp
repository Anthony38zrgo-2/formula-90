#pragma once

#include <cmath>
#include <godot_cpp/variant/variant.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/dictionary.hpp>

namespace godot {

inline Variant normalize_physical_document_numbers(const Variant &value) {
    if (value.get_type() == Variant::DICTIONARY) {
        const Dictionary source = value;
        Dictionary document;
        const Array keys = source.keys();
        for (int64_t index = 0; index < keys.size(); ++index) {
            document[keys[index]] = normalize_physical_document_numbers(source[keys[index]]);
        }
        return document;
    }
    if (value.get_type() == Variant::ARRAY) {
        const Array source = value;
        Array elements;
        elements.resize(source.size());
        for (int64_t index = 0; index < source.size(); ++index) {
            elements[index] = normalize_physical_document_numbers(source[index]);
        }
        return elements;
    }
    if (value.get_type() == Variant::FLOAT) {
        const double number = value;
        if (std::isfinite(number) && number == std::trunc(number) && std::abs(number) <= 9007199254740991.0) {
            return static_cast<int64_t>(number);
        }
    }
    return value;
}

}
