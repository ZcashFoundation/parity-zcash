extern crate bitcrypto as crypto;

fn target(data: &[u8]) {
    crypto::ripemd160(data);
    crypto::sha1(data);
    crypto::sha256(data);
    crypto::dhash160(data);
    crypto::dhash256(data);
    crypto::checksum(data);
};

#[cfg(feature = "afl")]
#[macro_use] extern crate afl;
#[cfg(feature = "afl")]
fn main() {
    fuzz!(|data| {
        target(&data);
    });
}

#[cfg(feature = "honggfuzz")]
#[macro_use] extern crate honggfuzz;
#[cfg(feature = "honggfuzz")]
fn main() {
    loop {
        fuzz!(|data| {
            target(data);
        });
    }
}
