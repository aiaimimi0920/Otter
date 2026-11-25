//! Test-only readback of the original swap chain's submitted logical buffer; never create replacement render targets.
use super::*;
impl Composition {
    pub(in crate::desktop) fn readback_for_test(
        &self,
    ) -> crate::desktop::presentation_readback_test::Pixels {
        self.ensure_attached().unwrap();
        unsafe {
            let desc = self.state.chain.GetDesc1().unwrap();
            assert_eq!(desc.BufferCount, 2);
            assert_eq!(desc.SwapEffect, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL);
            // D3D11 GetBuffer indices rotate logically with Present; original D2D backing/index 0 already points to the next frame,
            // unlike D3D12 physical indices. Three independent flip_probe colors confirm index 1 retains this submission.
            let texture: ID3D11Texture2D = self.state.chain.GetBuffer(1).unwrap();
            let device: ID3D11Device = self.state.chain.GetDevice().unwrap();
            crate::desktop::presentation_readback_test::texture(&device, &texture)
        }
    }
}
