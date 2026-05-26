use clap::crate_version;
use maud::{DOCTYPE, Markup, Render, html};

use crate::web::handlers;

pub fn page(title: &str, body: impl Render) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(title))
            body {
                (navbar())
                (body)
                (footer())
            }
        }
    }
}

fn head(title: &str) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { (title) }
            link
                rel="icon"
                href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='1em' font-size='90'>🦊</text></svg>";
            link
                rel="stylesheet"
                href="https://cdnjs.cloudflare.com/ajax/libs/bulma/1.0.4/css/bulma.min.css"
                integrity="sha512-yh2RE0wZCVZeysGiqTwDTO/dKelCbS9bP2L94UvOFtl/FKXcNAje3Y2oBg/ZMZ3LS1sicYk4dYVGtDex75fvvA=="
                crossorigin="anonymous"
                referrerpolicy="no-referrer";
            link
                rel="stylesheet"
                href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/7.0.1/css/all.min.css"
                integrity="sha512-2SwdPD6INVrV/lHTZbO2nodKhrnDdJK9/kg2XD1r9uGqPo1cUbujc+IYdlYdEErWNu69gVcYgdxlmVmzTWnetw=="
                crossorigin="anonymous"
                referrerpolicy="no-referrer";
            style {
                // language=css
                ".has-plotters-fix svg { width: 100%; height: auto; display: block; }"
            }
        }
    }
}

fn navbar() -> Markup {
    html! {
        nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
            div.container {
                div.navbar-brand {
                    a.navbar-item href="/" {
                        svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100" {
                            text y="0.95em" font-size="90" { "🦊" }
                        }
                    }
                    a.navbar-item href=(handlers::energy_balance::PATH) { "Energy profile" }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer.footer {
            div.content.has-text-centered {
                p {
                    strong { "Fennec" }
                    " "
                    a href=(format!("https://github.com/eigenein/fennec/releases/tag/{}", crate_version!())) {
                        (crate_version!())
                    }
                }
            }
        }
    }
}
