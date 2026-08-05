#!/usr/bin/env python
import os

env = SConscript("third_party/godot-cpp/SConstruct")
env.Append(CPPPATH=["native/include"])
standard_prefix = "/std:" if env["platform"] == "windows" else "-std="
env["CXXFLAGS"] = [flag for flag in env["CXXFLAGS"] if not str(flag).startswith(standard_prefix)]
env.Append(CXXFLAGS=["/std:c++20"] if env["platform"] == "windows" else ["-std=c++20"])

sources = Glob("native/src/*.cpp")
for folder in ["core", "vehicle", "camera", "presentation", "ui", "audio"]:
    sources += Glob("native/src/%s/*.cpp" % folder)

suffix = env["suffix"]
target = "game/addons/formula90s/bin/libformula90s" + suffix + env["SHLIBSUFFIX"]
library = env.SharedLibrary(target=target, source=sources)
Default(library)
