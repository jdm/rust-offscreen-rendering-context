//! A context abstraction that can switch between hardware and software rendering.

use crate::gl::types::GLuint;
use crate::platform::default::context::Context as HWContext;
use crate::platform::default::context::ContextDescriptor as HWContextDescriptor;
use crate::platform::default::surface::NativeWidget;
use crate::platform::default::device::Device as HWDevice;
use crate::platform::generic::osmesa::context::Context as OSMesaContext;
use crate::platform::generic::osmesa::context::ContextDescriptor as OSMesaContextDescriptor;
use crate::platform::generic::osmesa::device::Device as OSMesaDevice;
use crate::{ContextAttributes, ContextID, Error, SurfaceAccess, SurfaceID, SurfaceType};
use super::device::Device;
use super::surface::{Surface, SurfaceRef};

use euclid::default::Size2D;
use std::os::raw::c_void;

pub enum Context {
    Hardware(HWContext),
    Software(OSMesaContext),
}

#[derive(Clone)]
pub enum ContextDescriptor {
    Hardware(HWContextDescriptor),
    Software(OSMesaContextDescriptor),
}

impl Context {
}

impl Device {
    pub fn create_context_descriptor(&self, attributes: &ContextAttributes)
                                     -> Result<ContextDescriptor, Error> {
        match *self {
            Device::Hardware(ref device) => {
                device.create_context_descriptor(attributes).map(ContextDescriptor::Hardware)
            }
            Device::Software(ref device) => {
                device.create_context_descriptor(attributes).map(ContextDescriptor::Software)
            }
        }
    }

    /// Opens the device and context corresponding to the current hardware context.
    pub unsafe fn from_current_hardware_context() -> Result<(Device, Context), Error> {
        HWDevice::from_current_context().map(|(device, context)| {
            (Device::Hardware(device), Context::Hardware(context))
        })
    }

    /// Opens the device and context corresponding to the current software context.
    pub unsafe fn from_current_software_context() -> Result<(Device, Context), Error> {
        OSMesaDevice::from_current_context().map(|(device, context)| {
            (Device::Software(device), Context::Software(context))
        })
    }

    pub fn create_context(&mut self, descriptor: &ContextDescriptor) -> Result<Context, Error> {
        match (&mut *self, descriptor) {
            (&mut Device::Hardware(ref mut device),
             &ContextDescriptor::Hardware(ref descriptor)) => {
                 device.create_context(descriptor).map(Context::Hardware)
            }
            (&mut Device::Software(ref mut device),
             &ContextDescriptor::Software(ref descriptor)) => {
                device.create_context(descriptor).map(Context::Software)
            }
            _ => Err(Error::IncompatibleContextDescriptor),
        }
    }

    pub fn destroy_context(&self, context: &mut Context) -> Result<(), Error> {
        match (self, &mut *context) {
            (&Device::Hardware(ref device), &mut Context::Hardware(ref mut context)) => {
                device.destroy_context(context)
            }
            (&Device::Software(ref device), &mut Context::Software(ref mut context)) => {
                device.destroy_context(context)
            }
            _ => Err(Error::IncompatibleContext),
        }
    }

    pub fn context_descriptor(&self, context: &Context) -> ContextDescriptor {
        match (self, context) {
            (&Device::Hardware(ref device), &Context::Hardware(ref context)) => {
                ContextDescriptor::Hardware(device.context_descriptor(context))
            }
            (&Device::Software(ref device), &Context::Software(ref context)) => {
                ContextDescriptor::Software(device.context_descriptor(context))
            }
            _ => panic!("Incompatible context!"),
        }
    }

    pub fn make_context_current(&self, context: &Context) -> Result<(), Error> {
        match (self, context) {
            (&Device::Hardware(ref device), &Context::Hardware(ref context)) => {
                device.make_context_current(context)
            }
            (&Device::Software(ref device), &Context::Software(ref context)) => {
                device.make_context_current(context)
            }
            _ => Err(Error::IncompatibleContext),
        }
    }

    pub fn make_no_context_current(&self) -> Result<(), Error> {
        match self {
            &Device::Hardware(ref device) => {
                device.make_no_context_current()
            }
            &Device::Software(ref device) => {
                device.make_no_context_current()
            }
        }
    }

    pub fn bind_surface_to_context(&self, context: &mut Context, surface: Surface)
                                   -> Result<(), Error> {
        match (self, &mut *context) {
            (&Device::Hardware(ref device), &mut Context::Hardware(ref mut context)) => {
                match surface {
                    Surface::Hardware(surface) => device.bind_surface_to_context(context, surface),
                    _ => Err(Error::IncompatibleSurface),
                }
            }
            (&Device::Software(ref device), &mut Context::Software(ref mut context)) => {
                match surface {
                    Surface::Software(surface) => device.bind_surface_to_context(context, surface),
                    _ => Err(Error::IncompatibleSurface),
                }
            }
            _ => Err(Error::IncompatibleContext),
        }
    }

    pub fn unbind_surface_from_context(&self, context: &mut Context)
                                       -> Result<Option<Surface>, Error> {
        match (self, &mut *context) {
            (&Device::Hardware(ref device), &mut Context::Hardware(ref mut context)) => {
                device.unbind_surface_from_context(context).map(|surface| {
                    surface.map(Surface::Hardware)
                })
            }
            (&Device::Software(ref device), &mut Context::Software(ref mut context)) => {
                device.unbind_surface_from_context(context).map(|surface| {
                    surface.map(Surface::Software)
                })
            }
            _ => Err(Error::IncompatibleContext),
        }
    }

    pub fn context_descriptor_attributes(&self, context_descriptor: &ContextDescriptor)
                                         -> ContextAttributes {
        match (self, context_descriptor) {
            (&Device::Hardware(ref device),
             &ContextDescriptor::Hardware(ref context_descriptor)) => {
                device.context_descriptor_attributes(context_descriptor)
            }
            (&Device::Software(ref device),
             &ContextDescriptor::Software(ref context_descriptor)) => {
                device.context_descriptor_attributes(context_descriptor)
            }
            _ => panic!("Incompatible context!")
        }
    }

    pub fn get_proc_address(&self, context: &Context, symbol_name: &str) -> *const c_void {
        match (self, context) {
            (&Device::Hardware(ref device), &Context::Hardware(ref context)) => {
                device.get_proc_address(context, symbol_name)
            }
            (&Device::Software(ref device), &Context::Software(ref context)) => {
                device.get_proc_address(context, symbol_name)
            }
            _ => panic!("Incompatible context!"),
        }
    }

    pub fn context_id(&self, context: &Context) -> ContextID {
        match (self, context) {
            (&Device::Hardware(ref device), &Context::Hardware(ref context)) => {
                device.context_id(context)
            }
            (&Device::Software(ref device), &Context::Software(ref context)) => {
                device.context_id(context)
            }
            _ => panic!("Incompatible context!"),
        }
    }

    pub fn context_surface<'c>(&self, context: &'c Context) -> Result<Option<SurfaceRef<'c>>, Error> {
        match (self, context) {
            (Device::Hardware(ref device), Context::Hardware(ref context)) =>
                device.context_surface(context).map(|s| s.map(SurfaceRef::Hardware)),
            (Device::Software(ref device), Context::Software(ref context)) =>
                device.context_surface(context).map(|s| s.map(SurfaceRef::Software)),
            _ => Err(Error::UnsupportedOnThisPlatform),
        }
    }
}
