//! 仅测试成功Present后的上一物理buffer；不能把下一空buffer或重新栅格化源图冒充提交结果。
use super::*;
use windows::Win32::Graphics::Direct3D11::*;
struct Acquired<'a>(&'a State, &'a ID3D11Resource);
impl Drop for Acquired<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0
                .device
                .bridge
                .ReleaseWrappedResources(&[Some(self.1.clone())]);
            self.0.device.immediate.Flush();
        }
    }
}
impl CompositionD3d12 {
    pub(in crate::desktop) fn readback_for_test(
        &self,
    ) -> crate::desktop::presentation_readback_test::Pixels {
        let s = &*self.0;
        s.ensure_attached().unwrap();
        assert!(!s.ready);
        assert_eq!(s.buffers.len(), 2);
        let _frame = s.device.begin_frame().unwrap();
        unsafe {
            let index = (s.chain.GetCurrentBackBufferIndex() as usize + 1) % 2;
            let wrapped = &s.buffers[index].wrapped;
            s.device
                .bridge
                .AcquireWrappedResources(&[Some(wrapped.clone())]);
            let _acquired = Acquired(s, wrapped);
            crate::desktop::presentation_readback_test::texture(
                &s.device.device11,
                &wrapped.cast::<ID3D11Texture2D>().unwrap(),
            )
        }
    }
}
