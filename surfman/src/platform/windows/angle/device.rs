// surfman/surfman/src/platform/windows/angle/device.rs
//
//! A thread-local handle to the device.

use crate::egl::types::{EGLAttrib, EGLint};
use crate::egl;
use crate::platform::generic::egl::device::{EGL_FUNCTIONS, NativeDisplay, OwnedEGLDisplay};
use crate::platform::generic::egl::ffi::{EGL_D3D11_DEVICE_ANGLE, EGL_EXTENSION_FUNCTIONS};
use crate::platform::generic::egl::ffi::{EGL_NO_DEVICE_EXT, EGL_PLATFORM_DEVICE_EXT};
use crate::{Error, GLApi};
use super::adapter::Adapter;
use super::connection::Connection;

use std::os::raw::c_void;
use std::ptr;
use winapi::Interface;
use winapi::shared::dxgi::IDXGIDevice;
use winapi::shared::winerror;
use winapi::um::d3d11::{self, D3D11CreateDevice, D3D11_SDK_VERSION, ID3D11Device};
use winapi::um::d3dcommon::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL_9_3};
use wio::com::ComPtr;

pub struct Device {
    pub(crate) native_display: Box<dyn NativeDisplay>,
    pub(crate) d3d11_device: ComPtr<ID3D11Device>,
    pub(crate) d3d_driver_type: D3D_DRIVER_TYPE,
}

impl Device {
    #[allow(non_snake_case)]
    pub fn new(_: &Connection, adapter: &Adapter) -> Result<Device, Error> {
        let d3d_driver_type = adapter.d3d_driver_type;
        unsafe {
            let mut d3d11_device = ptr::null_mut();
            let mut d3d11_feature_level = 0;
            let mut d3d11_device_context = ptr::null_mut();
            let result = D3D11CreateDevice(adapter.dxgi_adapter.as_raw(),
                                           d3d_driver_type,
                                           ptr::null_mut(),
                                           //d3d11::D3D11_CREATE_DEVICE_DEBUG,
                                           0,
                                           ptr::null_mut(),
                                           0,
                                           D3D11_SDK_VERSION,
                                           &mut d3d11_device,
                                           &mut d3d11_feature_level,
                                           &mut d3d11_device_context);
            if !winerror::SUCCEEDED(result) {
                return Err(Error::DeviceOpenFailed);
            }
            debug_assert!(d3d11_feature_level >= D3D_FEATURE_LEVEL_9_3);
            let d3d11_device = ComPtr::from_raw(d3d11_device);
            Ok(Self::from_d3d11_device(d3d11_device, d3d_driver_type))
        }
    }
    
    pub fn from_d3d11_device(d3d11_device: ComPtr<ID3D11Device>, d3d_driver_type: D3D_DRIVER_TYPE) -> Self {
        unsafe {
            let eglCreateDeviceANGLE =
                EGL_EXTENSION_FUNCTIONS.CreateDeviceANGLE
                                       .expect("Where's the `EGL_ANGLE_device_creation` \
                                                extension?");
            let egl_device = eglCreateDeviceANGLE(EGL_D3D11_DEVICE_ANGLE as EGLint,
                                                  d3d11_device.as_raw() as *mut c_void,
                                                  ptr::null_mut());
            assert_ne!(egl_device, EGL_NO_DEVICE_EXT);

            EGL_FUNCTIONS.with(|egl| {
                let attribs = [egl::NONE as EGLAttrib, egl::NONE as EGLAttrib, 0, 0];
                let egl_display = egl.GetPlatformDisplay(EGL_PLATFORM_DEVICE_EXT,
                                                         egl_device as *mut c_void,
                                                         &attribs[0]);
                assert_ne!(egl_display, egl::NO_DISPLAY);
                let native_display = Box::new(OwnedEGLDisplay { egl_display });

                // I don't think this should ever fail.
                let (mut major_version, mut minor_version) = (0, 0);
                let result = egl.Initialize(native_display.egl_display(),
                                            &mut major_version,
                                            &mut minor_version);
                assert_ne!(result, egl::FALSE);

                Device { native_display, d3d11_device, d3d_driver_type }
            })
        }
    }

    #[inline]
    pub fn connection(&self) -> Connection {
        Connection
    }

    pub fn d3d11_device(&self) -> ComPtr<ID3D11Device> {
        self.d3d11_device.clone()
    }

    pub fn adapter(&self) -> Adapter {
        unsafe {
            let mut dxgi_device: *mut IDXGIDevice = ptr::null_mut();
            let result = (*self.d3d11_device).QueryInterface(
                &IDXGIDevice::uuidof(),
                &mut dxgi_device as *mut *mut IDXGIDevice as *mut *mut c_void);
            assert!(winerror::SUCCEEDED(result));
            let dxgi_device = ComPtr::from_raw(dxgi_device);

            let mut dxgi_adapter = ptr::null_mut();
            let result = (*dxgi_device).GetAdapter(&mut dxgi_adapter);
            assert!(winerror::SUCCEEDED(result));
            let dxgi_adapter = ComPtr::from_raw(dxgi_adapter);

            Adapter { dxgi_adapter, d3d_driver_type: self.d3d_driver_type }
        }
    }

    #[inline]
    pub fn gl_api() -> GLApi {
        GLApi::GLES
    }
}
