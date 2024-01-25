use maud::{html, DOCTYPE};

pub fn maud_page(content: maud::Markup) -> maud::Markup {
    html! {
       (DOCTYPE)
       (maud_header())
       (maud_body(content))
    }
}

fn maud_header() -> maud::Markup {
    html! {
        link rel="stylesheet" href="static/tailwind.css";
    }
}

fn maud_nav() -> maud::Markup {
    html! {
        nav class="nav bg-gray-100" {

            div class="flex lg:flex-1 m-3" {
                a class="no-underline hover:no-underline font-extrabold m-3 text-2xl" href="/" { "Timeline" }

            };
        }
    }
}

fn maud_body(content: maud::Markup) -> maud::Markup {
    html! {
        body {
            (maud_nav())
            div class="container w-full md:max-w-3xl mx-auto pt-20 place-content-center" {

                div class="w-full px-4 md:px-6  leading-normal" {
                    div class="span8" {
                        (content)

                    };
                };
            };
        };
    }
}
