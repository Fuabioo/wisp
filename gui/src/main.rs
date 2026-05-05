// wisp-admin — Pop!_OS COSMIC-native admin GUI for the wisp daemon.
//
// Architecture: see ../docs/adr/0002-cosmic-admin-gui.md

mod app;
mod backend;
mod components;
mod pages;
mod settings;
mod subscriptions;
mod theme;

fn main() -> cosmic::iced::Result {
    use tracing_subscriber::EnvFilter;

    // Always-on silencing for libcosmic's theme-config errors (they fire on
    // every startup against a stock COSMIC config that's missing keys our
    // libcosmic version expects — not actionable from our code) and the
    // benign wayland xdg_toplevel_icon_manager_v1 unsupported warning.
    // Applied on top of whatever the base filter resolves to so that
    // setting RUST_LOG doesn't accidentally re-enable the noise.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("cosmic::theme=off".parse().unwrap())
        .add_directive("cosmic::app::cosmic=off".parse().unwrap())
        .add_directive("winit_wayland::window::state=error".parse().unwrap())
        // tiny_skia (resvg's painter) warns whenever a gradient's
        // endpoints collapse to a single point or a horizontal/vertical
        // line — happens at the loop boundary of our pre-baked SVG
        // frames. Visually harmless, log-spammy.
        .add_directive("tiny_skia=error".parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(filter).init();

    cosmic::app::run::<app::WispAdmin>(cosmic::app::Settings::default(), ())
}
