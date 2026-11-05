//! 仅测试读取原交换链已提交的逻辑buffer；不新建替代渲染目标。
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
            // D3D11的GetBuffer索引随Present逻辑轮转；原D2D backing/索引0已指向下一帧，
            // 不同于D3D12的物理索引。flip_probe三次独立颜色确认索引1保本次提交。
            let texture: ID3D11Texture2D = self.state.chain.GetBuffer(1).unwrap();
            let device: ID3D11Device = self.state.chain.GetDevice().unwrap();
            crate::desktop::presentation_readback_test::texture(&device, &texture)
        }
    }
}
