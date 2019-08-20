extern crate zebra-crypto;

fn target(data: &[u8]) {
    zebra-crypto::ripemd160(data);
    zebra-crypto::sha1(data);
    zebra-crypto::sha256(data);
    zebra-crypto::dhash160(data);
    zebra-crypto::dhash256(data);
    zebra-crypto::checksum(data);
};


#[cfg(feature = "libfuzzer")]
#[macro_use] extern crate libfuzzer_sys;
#[cfg(feature = "libfuzzer")]
fuzz_target!(|data: &[u8]| {
    target(&data);
});

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
