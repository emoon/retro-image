//! The size limit is process-wide, so everything that changes it lives in
//! this one test, in its own test binary.

use retro_image::{DecodeError, Limits};

/// 1x1 single-plane ILBM: BMHD, CMAP (2 colors), BODY (one word).
fn tiny_ilbm() -> Vec<u8> {
    let mut ilbm = Vec::new();
    ilbm.extend_from_slice(b"FORM\0\0\0\x3eILBM");
    ilbm.extend_from_slice(b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01");
    ilbm.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
    ilbm.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
    ilbm
}

#[test]
fn lowering_the_limit_rejects_pictures_that_no_longer_fit() {
    let data = tiny_ilbm();
    assert!(retro_image::decode("x.iff", &data).is_ok());

    // A 1x1 picture takes 4 bytes under the limit's accounting. The decoder
    // also expands the 15 padding bits of each bitplane row, so it needs 64.
    Limits::default().with_max_image_bytes(64).install();
    let result = retro_image::decode("x.iff", &data);
    assert!(result.is_ok(), "{result:?}");

    Limits::default().with_max_image_bytes(3).install();
    let error = retro_image::decode("x.iff", &data).unwrap_err();
    let DecodeError::NoMatch { attempts } = &error else {
        panic!("expected NoMatch, got {error:?}");
    };
    assert!(
        attempts
            .iter()
            .any(|a| a.format().name == "Interchange File Format"
                && *a.error() == DecodeError::TooLarge),
        "the IFF decoder reports the size, not a bad file: {attempts:?}"
    );

    Limits::default().install();
    assert_eq!(Limits::current(), Limits::default());
    assert!(retro_image::decode("x.iff", &data).is_ok());
}
