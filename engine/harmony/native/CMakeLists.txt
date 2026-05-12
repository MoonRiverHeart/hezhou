cmake_minimum_required(VERSION 3.10)
project(hezhou_native)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# HarmonyOS SDK paths
set(OHOS_SDK_ROOT $ENV{OHOS_SDK_ROOT})
if(NOT OHOS_SDK_ROOT)
    message(FATAL_ERROR "OHOS_SDK_ROOT environment variable not set")
endif()

# Target architecture
set(TARGET_ARCH "aarch64-linux-ohos")
set(OHOS_SYSROOT "${OHOS_SDK_ROOT}/native/sysroot")

# Rust static library
set(RUST_BUILD_TYPE "release")
set(RUST_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../target/${TARGET_ARCH}/${RUST_BUILD_TYPE}/libhezhou_harmony.a")

if(NOT EXISTS ${RUST_LIB})
    message(WARNING "Rust library not found: ${RUST_LIB}. Run 'cargo build --target ${TARGET_ARCH} --release' first")
endif()

# Source files
add_library(hezhou_native SHARED
    hezhou_native.cpp
)

# Include directories
target_include_directories(hezhou_native PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${OHOS_SYSROOT}/usr/include
    ${OHOS_SDK_ROOT}/native/include
)

# Link libraries
target_link_libraries(hezhou_native
    ${RUST_LIB}
    
    # HarmonyOS native libraries
    ${OHOS_SYSROOT}/usr/lib/${TARGET_ARCH}/libace_napi.z.so
    ${OHOS_SYSROOT}/usr/lib/${TARGET_ARCH}/libnative_window.so
    
    # System libraries
    -llog
)

# Compiler flags
target_compile_options(hezhou_native PRIVATE
    -fvisibility=hidden
    -Wall
    -Wextra
)

# Linker flags
target_link_options(hezhou_native PRIVATE
    -Wl,--no-undefined
)

# Output directory
set_target_properties(hezhou_native PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../../../entry/libs/${TARGET_ARCH}"
)