#!/usr/bin/env python
import os
import subprocess

env = SConscript("third_party/godot-cpp/SConstruct")
env.Append(CPPPATH=["native/include"])
standard_prefix = "/std:" if env["platform"] == "windows" else "-std="
env["CXXFLAGS"] = [flag for flag in env["CXXFLAGS"] if not str(flag).startswith(standard_prefix)]
env.Append(CXXFLAGS=["/std:c++20"] if env["platform"] == "windows" else ["-std=c++20"])
build_sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
if len(build_sha) != 40:
    raise RuntimeError("Formula-90 BUILD must be a full git SHA")
env.Append(CPPDEFINES=[("FORMULA90_BUILD_SHA", '\\\"%s\\\"' % build_sha)])

sources = Glob("native/src/*.cpp")
for folder in ["core", "vehicle", "camera", "presentation", "ui", "audio", "sim"]:
    sources += Glob("native/src/%s/*.cpp" % folder)

suffix = env["suffix"]
target = "game/addons/formula90s/bin/libformula90s" + suffix + env["SHLIBSUFFIX"]
library = env.SharedLibrary(target=target, source=sources)
Default(library)
