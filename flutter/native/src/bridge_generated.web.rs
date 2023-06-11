use super::*;
// Section: wire functions

#[wasm_bindgen]
pub fn wire_say_hello(port_: MessagePort) {
    wire_say_hello_impl(port_)
}

#[wasm_bindgen]
pub fn wire_generate_qrcode(port_: MessagePort, amount: u8) {
    wire_generate_qrcode_impl(port_, amount)
}

#[wasm_bindgen]
pub fn wire_get_balance(port_: MessagePort) {
    wire_get_balance_impl(port_)
}

// Section: allocate functions

// Section: related functions

// Section: impl Wire2Api

// Section: impl Wire2Api for JsValue

impl Wire2Api<u8> for JsValue {
    fn wire2api(self) -> u8 {
        self.unchecked_into_f64() as _
    }
}
