//! Reacquire WGC factories after repeatedly exiting the final COM apartment; no desktop capture or capture sessions.
use super::*;

#[test]
fn factories_survive_complete_apartment_teardown_between_owners() {
    for _ in 0..3 {
        std::thread::spawn(|| {
            let _apartment = super::super::lifetime::Apartment::new().unwrap();
            assert!(supported().unwrap());
            let (_device, _context, winrt) = super::super::device().unwrap();
            let pool = pool(
                &winrt,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                SizeInt32 {
                    Width: 8,
                    Height: 8,
                },
            )
            .unwrap();
            pool.Close().unwrap();
        })
        .join()
        .unwrap();
    }
}
