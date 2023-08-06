use super::*;
// Section: wire functions

#[no_mangle]
pub extern "C" fn wire_init_cashu(port_: i64) {
    wire_init_cashu_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_get_cashu_balance(port_: i64) {
    wire_get_cashu_balance_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_cashu_mint_tokens(port_: i64, amount: u64, hash: *mut wire_uint_8_list) {
    wire_cashu_mint_tokens_impl(port_, amount, hash)
}

#[no_mangle]
pub extern "C" fn wire_get_cashu_mint_payment_request(port_: i64, amount: u64) {
    wire_get_cashu_mint_payment_request_impl(port_, amount)
}

#[no_mangle]
pub extern "C" fn wire_decode_invoice(invoice: *mut wire_uint_8_list) -> support::WireSyncReturn {
    wire_decode_invoice_impl(invoice)
}

#[no_mangle]
pub extern "C" fn wire_cashu_pay_invoice(port_: i64, invoice: *mut wire_uint_8_list) {
    wire_cashu_pay_invoice_impl(port_, invoice)
}

#[no_mangle]
pub extern "C" fn wire_join_federation(port_: i64, federation: *mut wire_uint_8_list) {
    wire_join_federation_impl(port_, federation)
}

#[no_mangle]
pub extern "C" fn wire_get_fedimint_payment_request(port_: i64, amount: u64) {
    wire_get_fedimint_payment_request_impl(port_, amount)
}

#[no_mangle]
pub extern "C" fn wire_fedimint_mint_tokens(
    port_: i64,
    amount: u64,
    operation_id: *mut wire_uint_8_list,
) {
    wire_fedimint_mint_tokens_impl(port_, amount, operation_id)
}

#[no_mangle]
pub extern "C" fn wire_get_fedimint_balance(port_: i64) {
    wire_get_fedimint_balance_impl(port_)
}

#[no_mangle]
pub extern "C" fn wire_fedimint_pay_invoice(port_: i64, invoice: *mut wire_uint_8_list) {
    wire_fedimint_pay_invoice_impl(port_, invoice)
}

#[no_mangle]
pub extern "C" fn wire_receive_token(port_: i64, token: *mut wire_uint_8_list) {
    wire_receive_token_impl(port_, token)
}

#[no_mangle]
pub extern "C" fn wire_get_btcprice(port_: i64) {
    wire_get_btcprice_impl(port_)
}

// Section: allocate functions

#[no_mangle]
pub extern "C" fn new_uint_8_list_0(len: i32) -> *mut wire_uint_8_list {
    let ans = wire_uint_8_list {
        ptr: support::new_leak_vec_ptr(Default::default(), len),
        len,
    };
    support::new_leak_box_ptr(ans)
}

// Section: related functions

// Section: impl Wire2Api

impl Wire2Api<String> for *mut wire_uint_8_list {
    fn wire2api(self) -> String {
        let vec: Vec<u8> = self.wire2api();
        String::from_utf8_lossy(&vec).into_owned()
    }
}

impl Wire2Api<Vec<u8>> for *mut wire_uint_8_list {
    fn wire2api(self) -> Vec<u8> {
        unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        }
    }
}
// Section: wire structs

#[repr(C)]
#[derive(Clone)]
pub struct wire_uint_8_list {
    ptr: *mut u8,
    len: i32,
}

// Section: impl NewWithNullPtr

pub trait NewWithNullPtr {
    fn new_with_null_ptr() -> Self;
}

impl<T> NewWithNullPtr for *mut T {
    fn new_with_null_ptr() -> Self {
        std::ptr::null_mut()
    }
}

// Section: sync execution mode utility

#[no_mangle]
pub extern "C" fn free_WireSyncReturn(ptr: support::WireSyncReturn) {
    unsafe {
        let _ = support::box_from_leak_ptr(ptr);
    };
}
