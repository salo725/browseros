// BraveWM - main.cpp: entry point.
// BraveOS v1.0.2 custom window manager: replaces Openbox. See wm/README.md.
#include "wm.hpp"

#include <getopt.h>
#include <signal.h>

volatile sig_atomic_t bwm_flag_restart = 0;
volatile sig_atomic_t bwm_flag_terminate = 0;

namespace bwm {
volatile sig_atomic_t g_sigterm_flag = 0;
volatile sig_atomic_t g_sigrestart_flag = 0;
}  // namespace bwm

static void on_signal(int sig) {
    if (sig == SIGUSR1) bwm_flag_restart = 1;   // restart (rc 42)
    else if (sig == SIGUSR2) bwm_flag_terminate = 1;  // clean logout (rc 0)
    else bwm_flag_terminate = 1;
}

int main(int argc, char **argv) {
    std::string config;
    bool version = false;
    int opt;
    while ((opt = getopt(argc, argv, "c:vh")) != -1) {
        switch (opt) {
            case 'c': config = optarg; break;
            case 'v': version = true; break;
            case 'h':
            default:
                fprintf(stderr,
                        "BraveWM - BraveOS window manager\n"
                        "usage: bravos-wm [-c config] [-v]\n");
                return opt == 'h' ? 0 : 1;
        }
    }
    if (version) {
        printf("bravos-wm 1.0.2\n");
        return 0;
    }

    signal(SIGUSR1, on_signal);
    signal(SIGUSR2, on_signal);
    signal(SIGTERM, on_signal);
    signal(SIGINT, on_signal);
    signal(SIGPIPE, SIG_IGN);

    bwm::Wm wm;
    if (!wm.init(config)) return 1;
    wm.run();
    // exit codes: 0 = clean session end, 42 = restart requested (SIGUSR1)
    if (bwm_flag_restart) return 42;
    return wm.wm_error ? 1 : 0;
}
