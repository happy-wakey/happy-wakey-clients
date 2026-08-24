cmake_minimum_required(VERSION 3.16)
project(happy_wakey_client C)
add_library(happy_wakey_client STATIC src/client.c)
target_include_directories(happy_wakey_client PUBLIC include)
target_compile_features(happy_wakey_client PUBLIC c_std_11)

