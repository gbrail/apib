package(default_visibility = ["//visibility:public"])

genrule(
    name = "cmake_config",
    srcs = glob(["*"]),
    outs = ["config.h"],
    cmd = "./external/libev/configure --srcdir=external/libev --disable-dependency-tracking && mv config.h $@",
)

cc_library(
    name = "libev",
    srcs = ["ev.c", "event.c", "config.h", "ev_vars.h", "ev_wrap.h"],
    hdrs = ["ev.h", "ev++.h", "event.h"],
    textual_hdrs = ["ev_epoll.c", "ev_select.c", "ev_poll.c", "ev_kqueue.c", "ev_port.c", "ev_linuxaio.c", "ev_iouring.c", "ev_win32.c"],
)
