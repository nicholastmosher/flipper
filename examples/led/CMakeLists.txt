cmake_minimum_required(VERSION 3.11)
project(led)

add_executable(rgb
        rgb.c
        )

add_library(flipper SHARED IMPORTED)

set_target_properties(flipper
        PROPERTIES
        IMPORTED_LOCATION /usr/local/lib/libflipper.so
        )

include_directories(rgb
        /usr/local/include
        ../../api/c/
        )

target_link_libraries(rgb
        flipper
        )
