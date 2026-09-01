cmake_minimum_required(VERSION 3.24)

project(f90_audio_dsp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(MSVC)
    add_compile_options(/W4 /WX /utf-8)
else()
    add_compile_options(-Wall -Wextra -Werror)
endif()

option(F90_DSP_BUILD_TESTS "Build f90_audio_dsp tests" ON)

# ---------------------------------------------------------------------------
# Fase 5: offline Faust code generation (no web compiler).
# The Faust executable is provisioned by scripts/setup_cpp_tools_windows.ps1
# (-Faust), which places it under <repo>/.tools/faust/bin/faust.exe together
# with its runtime DLLs, the std library (share/faust) and the C++ headers
# (include/faust). A missing/failing generator MUST stop the build.
# ---------------------------------------------------------------------------
set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
set(FAUST_EXE "" CACHE FILEPATH "Path to the faust.exe binary")
if(NOT FAUST_EXE)
    find_program(FAUST_EXE NAMES faust
        PATHS "${REPO_ROOT}/.tools/faust/bin"
        NO_DEFAULT_PATH)
endif()
if(NOT FAUST_EXE)
    message(FATAL_ERROR
        "faust.exe not found. Run: pwsh scripts/setup_cpp_tools_windows.ps1 -Faust")
endif()
set(FAUST_INCLUDE "${REPO_ROOT}/.tools/faust/include")
if(NOT EXISTS "${FAUST_INCLUDE}/faust/dsp/dsp.h")
    message(FATAL_ERROR
        "Faust C++ headers missing under ${FAUST_INCLUDE}. Re-run the Faust setup script.")
endif()

set(DSP_SRC "${CMAKE_CURRENT_SOURCE_DIR}/dsp/post_combustion.dsp")
set(GEN_DIR "${CMAKE_CURRENT_SOURCE_DIR}/generated")
set(GEN_CPP "${GEN_DIR}/post_combustion.cpp")
file(MAKE_DIRECTORY "${GEN_DIR}")

# Generate the C++ class from the .dsp source. COMMAND_ERROR_IS_FATAL makes any
# faust failure abort the build. The output is regenerated whenever the .dsp
# changes; it is #included by src/dsp_instance.cpp (never hand-edited).
add_custom_command(
    OUTPUT "${GEN_CPP}"
    COMMAND "${CMAKE_COMMAND}" -E echo "FASE5_FAUST_GEN: ${FAUST_EXE} -cn post_combustion -o ${GEN_CPP} ${DSP_SRC}"
    COMMAND "${FAUST_EXE}" -cn post_combustion -o "${GEN_CPP}" "${DSP_SRC}"
    DEPENDS "${DSP_SRC}"
    COMMENT "Generating post_combustion.cpp from Faust DSP (offline)"
    COMMAND_ERROR_IS_FATAL ANY
    VERBATIM
)
add_custom_target(faust_post_combustion_gen DEPENDS "${GEN_CPP}")

add_library(f90_audio_dsp SHARED
    src/c_api.cpp
    src/dsp_instance.cpp
    src/rt_alloc_guard.cpp
)
target_compile_definitions(f90_audio_dsp PRIVATE F90_AUDIO_DSP_EXPORTS F90_AUDIO_DSP_SHARED)
target_include_directories(f90_audio_dsp
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src
    PRIVATE ${GEN_DIR}
    PRIVATE ${FAUST_INCLUDE}
)
# dsp_instance.cpp #includes the generated Faust class; rebuild it if the
# generated source changes.
set_source_files_properties(src/dsp_instance.cpp PROPERTIES
    OBJECT_DEPENDS "${GEN_CPP}")
set_target_properties(f90_audio_dsp PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
    ARCHIVE_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/lib"
)
# The Faust-generated compute() has a large (unoptimized) stack frame in Debug;
# give the module a generous stack to avoid overflow during ctest.
if(MSVC)
    set_target_properties(f90_audio_dsp PROPERTIES LINK_FLAGS "/STACK:0x4000000")
endif()
add_dependencies(f90_audio_dsp faust_post_combustion_gen)

if(F90_DSP_BUILD_TESTS)
    enable_testing()
    add_executable(f90_audio_dsp_tests tests/test_main.cpp)
    target_link_libraries(f90_audio_dsp_tests PRIVATE f90_audio_dsp)
    target_include_directories(f90_audio_dsp_tests PRIVATE ${GEN_DIR} ${FAUST_INCLUDE})
    set_source_files_properties(tests/test_main.cpp PROPERTIES
        OBJECT_DEPENDS "${GEN_CPP}")
    set_target_properties(f90_audio_dsp_tests PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
    )
    if(MSVC)
        set_target_properties(f90_audio_dsp_tests PROPERTIES LINK_FLAGS "/STACK:0x4000000")
    endif()
    add_dependencies(f90_audio_dsp_tests faust_post_combustion_gen)
    add_test(NAME f90_audio_dsp_abi_suite COMMAND f90_audio_dsp_tests)
endif()
