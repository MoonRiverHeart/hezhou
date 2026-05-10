cmake_minimum_required(VERSION 3.4.1)
project(harmony_rust_demo)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})

set(RUST_LIB_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../libs/${OHOS_ARCH})

add_library(csharptorust_lib SHARED IMPORTED)
set_target_properties(csharptorust_lib PROPERTIES
    IMPORTED_LOCATION ${RUST_LIB_DIR}/libcsharptorust_lib.so
    IMPORTED_NO_SONAME TRUE
)

add_library(entry SHARED native_rust.cpp)
target_include_directories(entry PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(entry PUBLIC csharptorust_lib libace_napi.z.so)

set_target_properties(entry PROPERTIES
    LINK_FLAGS "-Wl,-rpath,./"
)
