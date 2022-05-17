use aba::journal::Currency;
use log::info;
use reqwasm::http::Request;
use yew::prelude::*;

enum Msg {
    AddOne,
    Currencies(Vec<Currency>),
}

struct Model {
    value: i64,
    currencies: String,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            value: i64::default(),
            currencies: String::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddOne => {
                self.value += 1;
                // the value has changed so we need to
                // re-render for it to appear on the page
                ctx.link().send_future(async {
                    let fetched_currencies: Vec<Currency> = Request::get("/api/ledger/currencies")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();
                    info!("currencies: {:?}", &fetched_currencies);
                    Msg::Currencies(fetched_currencies)
                });
                true
            }
            Msg::Currencies(currencies) => {
                self.currencies = currencies
                    .iter()
                    .cloned()
                    .map(|c| c.code)
                    .collect::<Vec<String>>()
                    .join(" ");
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // This gives us a component's "`Scope`" which allows us to send messages, etc to the component.
        let link = ctx.link();
        html! {
            <div class="container">
                <div class="block">
                    <p>{ self.value }</p>
                    <button class="button is-primary" onclick={link.callback(|_| Msg::AddOne)}>{ "+1" }</button>
                    <p>{ self.currencies.as_str() }</p>
                </div>
            </div>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<Model>();
}
