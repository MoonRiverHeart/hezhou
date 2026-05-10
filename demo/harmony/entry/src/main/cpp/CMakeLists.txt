cmake_minimum_required(VERSION 3.4.1)
project(harmony_rust_demo)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})

# 引入 Rust 编译的动态库 (SO)
add_library(rust_cdylib SHARED IMPORTED)
set_target_properties(rust_cdylib PROPERTIES IMPORTED_LOCATION
    ${CMAKE_CURRENT_SOURCE_DIR}/../libs/${OHOS_ARCH}/libcsharptorust_lib.so)

# 编译 NAPI 模块
add_library(entry SHARED native_rust.cpp)
target_include_directories(entry PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(entry PUBLIC rust_cdylib libace_napi.z.so)
