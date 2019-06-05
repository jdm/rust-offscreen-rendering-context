use euclid::Size2D;
use crate::platform::NativeGLContextMethods;
use crate::platform::with_egl::utils::{create_pixel_buffer_backed_offscreen_context};
use std::ops::Deref;
use std::os::raw;
use std::ptr;
use crate::egl;
use crate::egl::types::{EGLint, EGLBoolean, EGLDisplay, EGLSurface, EGLConfig, EGLContext};
use gleam::gl;
use crate::GLVersion;
use libloading as lib;

lazy_static! {
    static ref GL_LIB: Option<lib::Library>  = {
        let names = if cfg!(target_os="windows") {
            &["libEGL.dll"][..]
        } else {
            &["libGLESv2.so", "libGL.so", "libGLESv3.so"][..]
        };
        for name in names {
            if let Ok(lib) = lib::Library::new(name) {
                return Some(lib)
            }
        }

        None
    };
}
pub struct NativeGLContextHandle(pub EGLDisplay, pub EGLSurface);
unsafe impl Send for NativeGLContextHandle {}

pub struct NativeGLContext {
    native_display: EGLDisplay,
    native_surface: EGLSurface,
    native_context: EGLContext,
    egl: egl::Egl,
    weak: bool,
}

impl NativeGLContext {
    pub fn new(egl: egl::Egl,
               share_context: Option<&EGLContext>,
               display: EGLDisplay,
               surface: EGLSurface,
               config: EGLConfig,
               client_version: u8)
        -> Result<NativeGLContext, &'static str> {

        let shared = match share_context {
            Some(ctx) => *ctx,
            None => egl::NO_CONTEXT as EGLContext,
        };

        let attributes = [
            egl::CONTEXT_CLIENT_VERSION as EGLint, client_version as EGLint,
            egl::NONE as EGLint, 0, 0, 0, // see mod.rs
        ];

        let mut ctx =  unsafe { egl.CreateContext(display, config, shared, attributes.as_ptr()) };

        if share_context.is_some() && ctx == (egl::NO_CONTEXT as EGLContext) && client_version != 3 {
            // Workaround for GPUs that don't like different CONTEXT_CLIENT_VERSION value when sharing (e.g. Mali-T880).
            // Set CONTEXT_CLIENT_VERSION 3 to fix the shared ctx creation failure. Note that the ctx is still OpenGL ES 2.0
            // compliant because egl::OPENGL_ES2_BIT is set for egl::RENDERABLE_TYPE. See utils.rs.
            let attributes = [
                egl::CONTEXT_CLIENT_VERSION as EGLint, 3,
                egl::NONE as EGLint, 0, 0, 0, // see mod.rs
            ];
            ctx =  unsafe { egl.CreateContext(display, config, shared, attributes.as_ptr()) };
        }

        // TODO: Check for every type of error possible, not just client error?
        // Note if we do it we must do it too on egl::CreatePBufferSurface, etc...
        if ctx == (egl::NO_CONTEXT as EGLContext) {
            unsafe { egl.DestroySurface(display, surface) };
            return Err("Error creating an EGL context");
        }

        Ok(NativeGLContext {
            native_display: display,
            native_surface: surface,
            native_context: ctx,
            weak: false,
            egl,
        })
    }
}

impl Drop for NativeGLContext {
    fn drop(&mut self) {
        let _ = self.unbind();
        if !self.weak {
            unsafe {
                if self.egl.DestroySurface(self.native_display, self.native_surface) == 0 {
                    debug!("egl::DestroySurface failed");
                }
                if self.egl.DestroyContext(self.native_display, self.native_context) == 0 {
                    debug!("egl::DestroyContext failed");
                }
            }
        }
    }
}

impl NativeGLContextMethods for NativeGLContext {
    type Handle = NativeGLContextHandle;

    fn get_proc_address(addr: &str) -> *const () {
        get_proc_address(addr) as *const _
    }

    fn create_headless(api_type: &gl::GlType, api_version: GLVersion) -> Result<NativeGLContext, &'static str> {
        // We create a context with a dummy size, we can't rely on a
        // default framebuffer
        let egl = egl::Egl::load_with(get_proc_address);
        create_pixel_buffer_backed_offscreen_context(egl, Size2D::new(16, 16), None, api_type, api_version)
    }

    fn create_shared(with: Option<&Self::Handle>,
                     api_type: &gl::GlType,
                     api_version: GLVersion) -> Result<NativeGLContext, &'static str> {
        let egl = egl::Egl::load_with(get_proc_address);
        create_pixel_buffer_backed_offscreen_context(egl, Size2D::new(16, 16), with, api_type, api_version)
    }

    fn current_handle() -> Option<Self::Handle> {
        let egl = egl::Egl::load_with(get_proc_address);
        current_handle(&egl)
    }


    fn current() -> Option<Self> {
        let egl = egl::Egl::load_with(get_proc_address);
        if let Some(handle) = current_handle(&egl) {
            let surface = unsafe { egl.GetCurrentSurface(egl::DRAW as EGLint) };

            debug_assert!(surface != egl::NO_SURFACE);

            Some(NativeGLContext {
                native_context: handle.0,
                native_display: handle.1,
                native_surface: surface,
                weak: true,
                egl,
            })
        } else {
            None
        }
    }

    #[inline(always)]
    fn is_current(&self) -> bool {
        unsafe {
            self.egl.GetCurrentContext() == self.native_context
        }
    }

    fn make_current(&self) -> Result<(), &'static str> {
        unsafe {
            if !self.is_current() &&
                self.egl.MakeCurrent(self.native_display,
                                     self.native_surface,
                                     self.native_surface,
                                     self.native_context) == (egl::FALSE as EGLBoolean) {
                Err("egl::MakeCurrent")
            } else {
                Ok(())
            }
        }
    }

    fn handle(&self) -> Self::Handle {
        NativeGLContextHandle(self.native_context, self.native_display)
    }

    fn unbind(&self) -> Result<(), &'static str> {
        unsafe {
            if self.is_current() &&
               self.egl.MakeCurrent(self.native_display,
                                    egl::NO_SURFACE as EGLSurface,
                                    egl::NO_SURFACE as EGLSurface,
                                    egl::NO_CONTEXT as EGLContext) == (egl::FALSE as EGLBoolean) {
                Err("egl::MakeCurrent (on unbind)")
            } else {
                Ok(())
            }
        }
    }
}

fn current_handle(egl: &egl::Egl) -> Option<NativeGLContextHandle> {
    let native_context = unsafe { egl.GetCurrentContext() };
    let native_display = unsafe { egl.GetCurrentDisplay() };

    if native_context != egl::NO_CONTEXT && native_display != egl::NO_DISPLAY {
        Some(NativeGLContextHandle(native_context, native_display))
    } else {
        None
    }
}

// According to the EGL spec <= 1.4, eglGetProcAddress should only be used to
// retrieve extension functions. Some implementatios return NULL for core OpenGL functions.
// Other implementations may return non-NULL values even for invalid core or extension symbols.
// This is very dangerous, so we use dlsym function before calling eglGetProcAddress
// in order to avoid possible garbage pointers.
fn get_proc_address(addr: &str) -> *const raw::c_void {
    unsafe {
        if let Some(ref lib) = *GL_LIB {
            let symbol: Result<lib::Symbol<unsafe extern fn()>, _> = lib.get(addr.as_bytes());
            if let Ok(symbol) = symbol {
                return *symbol.deref() as *const _;
            }
        }
        ptr::null()
    }
}
