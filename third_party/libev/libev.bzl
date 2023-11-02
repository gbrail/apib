package(default_visibility = ["//visibility:public"])

# In case we ever do this again...
# This rule extracts the whole "libev" source tree (which is very small) and runs
# "configure" on that source tree to generate "config.h". Then it does the rest
# of the build the normal Bazel way (which is always always the easiest way to go!)
# Bazel has a set of rules that do "configure" and "make" all together, but for
# a simple library like this a hand-written BUILD file is a whole lot simpler and faster.
genrule(
    name = "cmake_config",
    srcs = glob(["*"]),
    outs = ["config.h"],
    # The source will be extracted two levels down, so we run it at the
    # top level for simplicity. We need the generated "config.h" file. However,
    # if we don't move it to the automagical destination "$@", then it will not
    # be able to be picked up by the larger Bazel build.
    cmd = "./external/libev/configure --srcdir=external/libev --disable-dependency-tracking && mv config.h $@",
)

cc_library(
    name = "libev",
    srcs = ["ev.c", "event.c", "config.h", "ev_vars.h", "ev_wrap.h"],
    hdrs = ["ev.h", "ev++.h", "event.h"],
    # These files won't compile standalone (and should not actually) but they are included
    # by "ev.c," which liberally uses ifdefs based on which types of back end are available.
    # Without this statement, bazel won't even make the files available to the build.
    textual_hdrs = ["ev_epoll.c", "ev_select.c", "ev_poll.c", "ev_kqueue.c", "ev_port.c", "ev_linuxaio.c", "ev_iouring.c", "ev_win32.c"],
)
