use super::*;
// Section: wire functions

#[wasm_bindgen]
pub fn wire_init_cashu(port_: MessagePort) {
    wire_init_cashu_impl(port_)
}

#[wasm_bindgen]
pub fn wire_get_cashu_balance(port_: MessagePort) {
    wire_get_cashu_balance_impl(port_)
}

#[wasm_bindgen]
pub fn wire_cashu_mint_tokens(port_: MessagePort, amount: u64, hash: String) {
    wire_cashu_mint_tokens_impl(port_, amount, hash)
}

#[wasm_bindgen]
pub fn wire_get_cashu_mint_payment_request(port_: MessagePort, amount: u64) {
    wire_get_cashu_mint_payment_request_impl(port_, amount)
}

#[wasm_bindgen]
pub fn wire_get_fedimint_payment_request(port_: MessagePort, amount: u64) {
    wire_get_fedimint_payment_request_impl(port_, amount)
}

#[wasm_bindgen]
pub fn wire_fedimint_mint_tokens(port_: MessagePort, amount: u64, operation_id: String) {
    wire_fedimint_mint_tokens_impl(port_, amount, operation_id)
}

#[wasm_bindgen]
pub fn wire_decode_invoice(port_: MessagePort, invoice: String) {
    wire_decode_invoice_impl(port_, invoice)
}

#[wasm_bindgen]
pub fn wire_pay_invoice(port_: MessagePort, invoice: String) {
    wire_pay_invoice_impl(port_, invoice)
}

#[wasm_bindgen]
pub fn wire_import_token(port_: MessagePort, token: String) {
    wire_import_token_impl(port_, token)
}

#[wasm_bindgen]
pub fn wire_join_federation(port_: MessagePort, federation: String) {
    wire_join_federation_impl(port_, federation)
}

// Section: allocate functions

// Section: related functions

// Section: impl Wire2Api

impl Wire2Api<String> for String {
    fn wire2api(self) -> String {
        self
    }
}

impl Wire2Api<Vec<u8>> for Box<[u8]> {
    fn wire2api(self) -> Vec<u8> {
        self.into_vec()
    }
}
// Section: impl Wire2Api for JsValue

impl Wire2Api<String> for JsValue {
    fn wire2api(self) -> String {
        self.as_string().expect("non-UTF-8 string, or not a string")
    }
}
impl Wire2Api<u64> for JsValue {
    fn wire2api(self) -> u64 {
        ::std::convert::TryInto::try_into(self.dyn_into::<js_sys::BigInt>().unwrap()).unwrap()
    }
}
impl Wire2Api<u8> for JsValue {
    fn wire2api(self) -> u8 {
        self.unchecked_into_f64() as _
    }
}
impl Wire2Api<Vec<u8>> for JsValue {
    fn wire2api(self) -> Vec<u8> {
        self.unchecked_into::<js_sys::Uint8Array>().to_vec().into()
    }
}
