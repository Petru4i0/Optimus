use crate::core::{
    c_void, iter, Cursor, Path, PCWSTR, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, DestroyIcon,
    DIB_RGB_COLORS, DrawIconEx, ExtractIconExW, HBRUSH, HGDIOBJ, HICON, ICON_CACHE_MAX_ITEMS,
    ICON_SIZE, ImageFormat, OnceLock, DI_NORMAL, Mutex, SelectObject,
};
use crate::types::AppError;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::os::windows::ffi::OsStrExt;
static ICON_LRU_CACHE: OnceLock<Mutex<LruCache<String, Vec<u8>>>> = OnceLock::new();

fn icon_cache() -> &'static Mutex<LruCache<String, Vec<u8>>> {
    ICON_LRU_CACHE.get_or_init(|| {
        let capacity = NonZeroUsize::new(ICON_CACHE_MAX_ITEMS).unwrap_or(NonZeroUsize::MIN);
        Mutex::new(LruCache::new(capacity))
    })
}

pub(crate) fn extract_icon_png_bytes(path: &Path) -> Result<Vec<u8>, AppError> {
    let key = path.to_string_lossy().into_owned();

    if let Ok(mut cache) = icon_cache().lock() {
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    let rgba = extract_icon_rgba(path, ICON_SIZE)?;
    let image = image::RgbaImage::from_raw(ICON_SIZE as u32, ICON_SIZE as u32, rgba)
        .ok_or_else(|| AppError::Message("failed to build RGBA image".to_owned()))?;

    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| AppError::Message(format!("failed to encode PNG: {e}")))?;

    let encoded = cursor.into_inner();
    if let Ok(mut cache) = icon_cache().lock() {
        cache.put(key, encoded.clone());
    }

    Ok(encoded)
}

pub(crate) fn extract_icon_rgba(path: &Path, icon_size: i32) -> Result<Vec<u8>, AppError> {
    unsafe {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let mut icon = HICON::default();
        let extracted = ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(std::ptr::addr_of_mut!(icon)),
            None,
            1,
        );

        if extracted == 0 || icon.is_invalid() {
            return Err(AppError::Message(format!(
                "no icon extracted for {}",
                path.display()
            )));
        }

        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("CreateCompatibleDC failed".to_owned()));
        }

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: icon_size,
                biHeight: -icon_size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let bitmap = match CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(dc);
                let _ = DestroyIcon(icon);
                return Err(AppError::Message("CreateDIBSection failed".to_owned()));
            }
        };

        if bits_ptr.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message(
                "CreateDIBSection returned null".to_owned(),
            ));
        }

        let old = SelectObject(dc, HGDIOBJ(bitmap.0));

        let drew = DrawIconEx(
            dc,
            0,
            0,
            icon,
            icon_size,
            icon_size,
            0,
            Some(HBRUSH(std::ptr::null_mut())),
            DI_NORMAL,
        )
        .is_ok();

        if !drew {
            if !old.is_invalid() {
                let _ = SelectObject(dc, old);
            }
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            let _ = DestroyIcon(icon);
            return Err(AppError::Message("DrawIconEx failed".to_owned()));
        }

        let count = (icon_size * icon_size * 4) as usize;
        let bgra = std::slice::from_raw_parts(bits_ptr as *const u8, count);
        let mut rgba = Vec::with_capacity(count);
        for px in bgra.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }

        if !old.is_invalid() {
            let _ = SelectObject(dc, old);
        }

        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        let _ = DestroyIcon(icon);

        Ok(rgba)
    }
}

