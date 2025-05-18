#![allow(unused)]
mod bindings_pdfium {
    #![allow(warnings)]
    include!(concat!(env!("OUT_DIR"), "/bindings_pdfium.rs"));
}

use parking_lot::{const_mutex, Mutex};
use static_assertions::assert_not_impl_any;
use std::ffi::{c_void, CStr};
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Library(PhantomData<*mut ()>);

assert_not_impl_any!(Library: Sync, Send);

static INITIALIZED: Mutex<bool> = const_mutex(false);

impl Drop for Library {
    fn drop(&mut self) {
        let mut initialized = INITIALIZED.lock();
        unsafe {
            bindings_pdfium::FPDF_DestroyLibrary();
        }
        *initialized = false;
    }
}

impl Library {

    pub fn init_library() -> Option<Library> {
        let mut initialized = INITIALIZED.lock();

        if *initialized {
            None
        } else {
            let config = bindings_pdfium::FPDF_LIBRARY_CONFIG_ {
                version: 2,
                m_pUserFontPaths: std::ptr::null::<*const i8>() as *mut _,
                m_pIsolate: std::ptr::null::<std::ffi::c_void>() as *mut _,
                m_v8EmbedderSlot: 0,
                m_pPlatform: std::ptr::null::<std::ffi::c_void>() as *mut _,
                m_RendererType: 0, // 0 = default
            };
            unsafe {
                bindings_pdfium::FPDF_InitLibraryWithConfig(&config);
            }
            *initialized = true;
            Some(Library(Default::default()))
        }
    }

    fn get_last_error(&self) -> Option<PdfiumError> {
        PdfiumError::from_code(unsafe { bindings_pdfium::FPDF_GetLastError() as u32 })
    }

    fn last_error(&self) -> PdfiumError {
        self.get_last_error().unwrap_or(PdfiumError::Unknown)
    }

    pub fn load_document<'library>(
        &'library self,
        path: &Path,
        password: Option<&CStr>,
    ) -> Result<DocumentHandle<'static, 'library>, PdfiumError> {
        let password = password.map(|x| x.as_ptr()).unwrap_or_else(std::ptr::null);

        let path = cstr(path)?;

        let handle = NonNull::new(unsafe { bindings_pdfium::FPDF_LoadDocument(path.as_ptr(), password) });

        handle
            .map(|handle| DocumentHandle {
                handle,
                data_life_time: Default::default(),
                library_life_time: Default::default(),
            })
            .ok_or_else(|| self.last_error())
    }

    pub fn load_document_from_bytes<'data, 'library>(
        &'library self,
        buffer: &'data [u8],
        password: Option<&CStr>,
    ) -> Result<DocumentHandle<'data, 'library>, PdfiumError> {
        let password = password.map(|x| x.as_ptr()).unwrap_or_else(std::ptr::null);

        let handle = NonNull::new(unsafe {
            bindings_pdfium::FPDF_LoadMemDocument(
                buffer.as_ptr() as *mut c_void,
                buffer.len() as i32,
                password,
            )
        });

        handle
            .map(|handle| DocumentHandle {
                handle,
                data_life_time: Default::default(),
                library_life_time: Default::default(),
            })
            .ok_or_else(|| self.last_error())
    }


    pub fn get_page_count(&self, document: &DocumentHandle) -> usize {
        unsafe { bindings_pdfium::FPDF_GetPageCount(document.handle.as_ptr()) as usize }
    }

    pub fn load_page<'data, 'library>(
        &'library self,
        document: &'data DocumentHandle,
        index: usize,
    ) -> Result<PageHandle<'data, 'library>, PdfiumError> {
        let handle = NonNull::new(unsafe {
            bindings_pdfium::FPDF_LoadPage(document.handle.as_ptr(), index as i32)
        });

        handle
            .map(|handle| PageHandle {
                handle,
                data_life_time: Default::default(),
                library_life_time: Default::default(),
            })
            .ok_or_else(|| self.last_error())
    }
    pub fn get_page_width(&self, page: &PageHandle) -> f32 {
        unsafe { bindings_pdfium::FPDF_GetPageWidthF(page.handle.as_ptr()) }
    }


    pub fn get_page_height(&self, page: &PageHandle) -> f32 {
        unsafe { bindings_pdfium::FPDF_GetPageHeightF(page.handle.as_ptr()) }
    }

    pub fn render_page_to_bitmap(
        &self,
        bitmap: &mut BitmapHandle,
        page: &PageHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        orientation: PageOrientation,
        flags: i32,
    ) {
        unsafe {
            bindings_pdfium::FPDF_RenderPageBitmap(
                bitmap.handle.as_ptr(),
                page.handle.as_ptr(),
                x,
                y,
                width,
                height,
                orientation as i32,
                flags,
            );
        }
    }


    pub fn create_bitmap<'library>(
        &'library self,
        width: usize,
        height: usize,
        format: BitmapFormat,
    ) -> Result<BitmapHandle<'static, 'library>, PdfiumError> {
        self.create_bitmap_ex(width, height, format, None, 0)
    }


    pub fn create_bitmap_from_buffer<'data, 'library>(
        &'library self,
        width: usize,
        height: usize,
        format: BitmapFormat,
        buffer: &'data mut [u8],
        height_stride: usize,
    ) -> Result<BitmapHandle<'data, 'library>, PdfiumError> {
        self.create_bitmap_ex(width, height, format, Some(buffer), height_stride)
    }

    fn create_bitmap_ex<'data, 'library>(
        &'library self,
        width: usize,
        height: usize,
        format: BitmapFormat,
        buffer: Option<&'data mut [u8]>,
        height_stride: usize,
    ) -> Result<BitmapHandle<'data, 'library>, PdfiumError> {
        let buffer = buffer
            .map(|buffer| {
                if buffer.len() < height * height_stride {
                    Err(PdfiumError::BadFormat)
                } else {
                    Ok(buffer.as_ptr())
                }
            })
            .transpose()?;

        let buffer = buffer.unwrap_or_else(std::ptr::null);

        let handle = NonNull::new(unsafe {
            bindings_pdfium::FPDFBitmap_CreateEx(
                width as i32,
                height as i32,
                format as i32,
                buffer as *mut c_void,
                height_stride as i32,
            )
        });

        handle
            .map(|handle| BitmapHandle {
                handle,
                data_life_time: Default::default(),
                library_life_time: Default::default(),
            })
            .ok_or_else(|| self.last_error())
    }

    pub fn get_bitmap_format(&self, bitmap: &BitmapHandle) -> BitmapFormat {
        let format = unsafe { bindings_pdfium::FPDFBitmap_GetFormat(bitmap.handle.as_ptr()) };

        BitmapFormat::from_i32(format).unwrap()
    }


    pub fn get_bitmap_width(&self, bitmap: &BitmapHandle) -> usize {
        unsafe { bindings_pdfium::FPDFBitmap_GetWidth(bitmap.handle.as_ptr()) as usize }
    }

    pub fn get_bitmap_height(&self, bitmap: &BitmapHandle) -> usize {
        unsafe { bindings_pdfium::FPDFBitmap_GetHeight(bitmap.handle.as_ptr()) as usize }
    }


    pub fn get_bitmap_stride(&self, bitmap: &BitmapHandle) -> usize {
        unsafe { bindings_pdfium::FPDFBitmap_GetStride(bitmap.handle.as_ptr()) as usize }
    }


    pub fn bitmap_fill_rect (
        &self,
        bitmap: &mut BitmapHandle,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: u64,
    ) {
        unsafe { bindings_pdfium::FPDFBitmap_FillRect(bitmap.handle.as_ptr(), x, y, width, height, color as u32); }
    }

    pub fn get_bitmap_buffer_mut<'a>(&self, bitmap: &'a mut BitmapHandle) -> &'a mut [u8] {
        let length = self.get_bitmap_buffer_length(bitmap);

        unsafe {
            std::slice::from_raw_parts_mut(
                bindings_pdfium::FPDFBitmap_GetBuffer(bitmap.handle.as_ptr()) as _,
                length,
            )
        }
    }

    pub fn get_bitmap_buffer<'a>(&self, bitmap: &'a BitmapHandle) -> &'a [u8] {
        let length = self.get_bitmap_buffer_length(bitmap);

        unsafe {
            std::slice::from_raw_parts(
                bindings_pdfium::FPDFBitmap_GetBuffer(bitmap.handle.as_ptr()) as _,
                length,
            )
        }
    }

    fn get_bitmap_buffer_length(&self, bitmap: &BitmapHandle) -> usize {
        let stride = self.get_bitmap_stride(bitmap);
        let line_width =
            self.get_bitmap_width(bitmap) * self.get_bitmap_format(bitmap).bytes_per_pixel();

        stride * self.get_bitmap_height(bitmap) - (stride - line_width)
    }
}

/// PDFium Error Codes
#[repr(i32)]
#[derive(PartialEq, Eq, Debug)]
pub enum PdfiumError {
    /// Unknown error.
    Unknown = bindings_pdfium::FPDF_ERR_UNKNOWN as i32,
    /// File not found or could not be opened.
    BadFile = bindings_pdfium::FPDF_ERR_FILE as i32,
    /// File not in PDF format or corrupted.
    BadFormat = bindings_pdfium::FPDF_ERR_FORMAT as i32,
    /// Password required or incorrect password.
    BadPassword = bindings_pdfium::FPDF_ERR_PASSWORD as i32,
    /// Unsupported security scheme.
    UnsupportedSecurityScheme = bindings_pdfium::FPDF_ERR_SECURITY as i32,
    /// Page not found or content error.
    BadPage = bindings_pdfium::FPDF_ERR_PAGE as i32,
}

impl PdfiumError {
    fn from_code(code: u32) -> Option<PdfiumError> {
        match code {
            bindings_pdfium::FPDF_ERR_SUCCESS => None,
            bindings_pdfium::FPDF_ERR_UNKNOWN => Some(PdfiumError::Unknown),
            bindings_pdfium::FPDF_ERR_FILE => Some(PdfiumError::BadFile),
            bindings_pdfium::FPDF_ERR_FORMAT => Some(PdfiumError::BadFormat),
            bindings_pdfium::FPDF_ERR_PASSWORD => Some(PdfiumError::BadPassword),
            bindings_pdfium::FPDF_ERR_SECURITY => Some(PdfiumError::UnsupportedSecurityScheme),
            bindings_pdfium::FPDF_ERR_PAGE => Some(PdfiumError::BadPage),
            _ => Some(PdfiumError::Unknown),
        }
    }
}

/// The format of pixels in the bitmap.
#[repr(i32)]
#[derive(Debug, PartialEq, Eq)]
pub enum BitmapFormat {
    /// Gray scale bitmap, one byte per pixel.
    GreyScale = bindings_pdfium::FPDFBitmap_Gray as i32,
    /// 3 bytes per pixel, byte order: blue, green, red.
    BGR = bindings_pdfium::FPDFBitmap_BGR as i32,
    /// 4 bytes per pixel, byte order: blue, green, red, unused.
    BGRx = bindings_pdfium::FPDFBitmap_BGRx as i32,
    /// 4 bytes per pixel, byte order: blue, green, red, alpha.
    BGRA = bindings_pdfium::FPDFBitmap_BGRA as i32,
}

impl BitmapFormat {

    pub fn bytes_per_pixel(&self) -> usize {
        match *self {
            BitmapFormat::GreyScale => 1,
            BitmapFormat::BGR => 3,
            BitmapFormat::BGRx | BitmapFormat::BGRA => 4,
        }
    }

    fn from_i32(number: i32) -> Option<BitmapFormat> {
        match number {
            x if x == BitmapFormat::GreyScale as i32 => Some(BitmapFormat::GreyScale),
            x if x == BitmapFormat::BGR as i32 => Some(BitmapFormat::BGR),
            x if x == BitmapFormat::BGRx as i32 => Some(BitmapFormat::BGRx),
            x if x == BitmapFormat::BGRA as i32 => Some(BitmapFormat::BGRA),
            _ => None,
        }
    }
}

/// Orientation to render the page.
pub enum PageOrientation {
    /// normal
    Normal = 0,
    /// rotated 90 degrees clockwise
    Clockwise = 1,
    /// rotated 180 degrees
    Flip = 2,
    /// rotated 90 degrees counter-clockwise
    CounterClockwise = 3,
}

pub mod rendering_flags {


    use super::bindings_pdfium;

    /// Normal display (No flags)
    pub const NORMAL: i32 = 0;

    /// Set if annotations are to be rendered.
    pub const ANNOTATIONS: i32 = bindings_pdfium::FPDF_ANNOT as i32;

    /// Set if using text rendering optimized for LCD display. This flag will only
    /// take effect if anti-aliasing is enabled for text.
    pub const LCD_TEXT: i32 = bindings_pdfium::FPDF_LCD_TEXT as i32;

    /// Don't use the native text output available on some platforms
    pub const NO_NATIVE_TEXT: i32 = bindings_pdfium::FPDF_NO_NATIVETEXT as i32;

    /// Grayscale output
    pub const GRAY_SCALE: i32 = bindings_pdfium::FPDF_GRAYSCALE as i32;

    /// Limit image cache size.
    pub const LIMITED_IMAGE_CACHE: i32 = bindings_pdfium::FPDF_RENDER_LIMITEDIMAGECACHE as i32;

    /// Always use halftone for image stretching.
    pub const FORCE_HALFTONE: i32 = bindings_pdfium::FPDF_RENDER_FORCEHALFTONE as i32;

    /// Render for printing.
    pub const PRINTING: i32 = bindings_pdfium::FPDF_PRINTING as i32;

    /// Set to disable anti-aliasing on text. This flag will also disable LCD
    /// optimization for text rendering.
    pub const NO_SMOOTH_TEXT: i32 = bindings_pdfium::FPDF_RENDER_NO_SMOOTHTEXT as i32;

    /// Set to disable anti-aliasing on images.
    pub const NO_SMOOTH_IMAGE: i32 = bindings_pdfium::FPDF_RENDER_NO_SMOOTHIMAGE as i32;

    /// Set to disable anti-aliasing on paths.
    pub const NO_SMOOTH_PATH: i32 = bindings_pdfium::FPDF_RENDER_NO_SMOOTHPATH as i32;

    /// Set whether to render in a reverse Byte order, this flag is only used when
    /// rendering to a bitmap.
    pub const REVERSE_BYTE_ORDER: i32 = bindings_pdfium::FPDF_REVERSE_BYTE_ORDER as i32;
}

pub struct DocumentHandle<'a, 'b> {
    handle: NonNull<bindings_pdfium::fpdf_document_t__>,
    data_life_time: PhantomData<&'a [u8]>,
    library_life_time: PhantomData<&'b Library>,
}

assert_not_impl_any!(DocumentHandle: Sync, Send);

impl Drop for DocumentHandle<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            bindings_pdfium::FPDF_CloseDocument(self.handle.as_ptr());
        }
    }
}

impl fmt::Debug for DocumentHandle<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DocumentHandle")
    }
}

pub struct PageHandle<'a, 'b> {
    handle: NonNull<bindings_pdfium::fpdf_page_t__>,
    data_life_time: PhantomData<&'a [u8]>,
    library_life_time: PhantomData<&'b Library>,
}

assert_not_impl_any!(PageHandle: Sync, Send);

impl Drop for PageHandle<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            bindings_pdfium::FPDF_ClosePage(self.handle.as_ptr());
        }
    }
}


pub struct BitmapHandle<'a, 'b> {
    handle: NonNull<bindings_pdfium::fpdf_bitmap_t__>,
    data_life_time: PhantomData<&'a mut [u8]>,
    library_life_time: PhantomData<&'b Library>,
}

assert_not_impl_any!(BitmapHandle: Sync, Send);

impl Drop for BitmapHandle<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            bindings_pdfium::FPDFBitmap_Destroy(self.handle.as_ptr());
        }
    }
}

use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[cfg(not(unix))]
fn cstr(path: &Path) -> Result<CString, PdfiumError> {
    let path = path.to_str().ok_or(PdfiumError::BadFile)?;
    CString::new(path).map_err(|_| PdfiumError::BadFile)
}

#[cfg(unix)]
fn cstr(path: &Path) -> Result<CString, PdfiumError> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| PdfiumError::BadFile)
}