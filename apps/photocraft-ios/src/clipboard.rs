//! PNG clipboard boundary. Services explicitly exchange straight RGBA8 pixels.
use photocraft_codecs::{ChannelLayout, DecodeOptions, EncodeOptions, Format, Image, Limits, SampleType};

const MAX_BYTES: u64 = 256 * 1024 * 1024;
fn limits() -> Limits {
    Limits { max_width: 32768, max_height: 32768, max_pixels: MAX_BYTES / 4, max_alloc: MAX_BYTES }
}

fn rgba_len(width: u32, height: u32, bound: &Limits) -> Result<usize, String> {
    bound.check(width, height, ChannelLayout::Rgba, SampleType::U8).map_err(|e| e.to_string())?;
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| "clipboard image size overflow".into())
}

fn encode_png(width: u32, height: u32, pixels: &[u8], bound: &Limits) -> Result<Vec<u8>, String> {
    if rgba_len(width, height, bound)? != pixels.len() {
        return Err("clipboard RGBA8 pixel length does not match dimensions".into());
    }
    let image = Image::from_u8(width, height, ChannelLayout::Rgba, pixels.to_vec()).map_err(|e| e.to_string())?;
    photocraft_codecs::encode(&image, Format::Png, &EncodeOptions::default()).map_err(|e| e.to_string())
}

fn decode_png(bytes: &[u8], bound: &Limits) -> Result<(u32, u32, Vec<u8>), String> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > bound.max_alloc {
        return Err("clipboard PNG exceeds byte limit".into());
    }
    let image = photocraft_codecs::decode_as_with(Format::Png, bytes, &DecodeOptions { limits: *bound, ..Default::default() }).map_err(|e| e.to_string())?;
    let length = rgba_len(image.width(), image.height(), bound)?;
    let pixels = image.to_rgba8();
    if pixels.len() != length {
        return Err("decoded clipboard RGBA8 length is invalid".into());
    }
    Ok((image.width(), image.height(), pixels))
}

#[cfg(target_os = "ios")]
pub fn install(services: &mut photocraft_ui_egui::Services) {
    services.encode_png = Some(Box::new(|w, h, pixels| encode_png(w, h, pixels, &limits())));
    services.clipboard_set_image = Some(Box::new(native::set_image));
    services.clipboard_get_image = Some(Box::new(|| match native::get_image() {
        Ok(image) => image,
        Err(error) => {
            log::warn!("PhotoCraft clipboard paste failed: {error}");
            None
        }
    }));
    services.open_url = Some(Box::new(native::open_url));
}

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
mod native {
    use super::*;
    use objc2::MainThreadMarker;
    use objc2_foundation::{NSData, NSDictionary, NSString, NSURL};
    use objc2_ui_kit::{UIApplication, UIPasteboard};

    fn main_thread() -> Result<MainThreadMarker, String> {
        MainThreadMarker::new().ok_or_else(|| "UIKit clipboard and URL services require the main thread".into())
    }
    pub(super) fn set_image(w: u32, h: u32, pixels: &[u8]) -> Result<(), String> {
        let _mtm = main_thread()?;
        let png = encode_png(w, h, pixels, &limits())?;
        UIPasteboard::generalPasteboard().setData_forPasteboardType(&NSData::with_bytes(&png), &NSString::from_str("public.png"));
        Ok(())
    }
    pub(super) fn get_image() -> Result<Option<(u32, u32, Vec<u8>)>, String> {
        let _mtm = main_thread()?;
        let Some(data) = UIPasteboard::generalPasteboard().dataForPasteboardType(&NSString::from_str("public.png")) else { return Ok(None) };
        if u64::try_from(data.length()).unwrap_or(u64::MAX) > MAX_BYTES {
            return Err("clipboard PNG exceeds byte limit".into());
        }
        decode_png(&data.to_vec(), &limits()).map(Some)
    }
    pub(super) fn open_url(value: &str) -> Result<(), String> {
        let mtm = main_thread()?;
        if value.is_empty() || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("invalid URL".into());
        }
        let url = NSURL::URLWithString(&NSString::from_str(value)).ok_or_else(|| "invalid URL".to_string())?;
        if url.scheme().is_none() {
            return Err("URL requires a scheme".into());
        }
        let app = UIApplication::sharedApplication(mtm);
        if !app.canOpenURL(&url) {
            return Err("no application can open this URL".into());
        }
        let completion = block2::RcBlock::new(|success: objc2::runtime::Bool| {
            if !success.as_bool() {
                log::warn!("PhotoCraft system URL request was rejected");
            }
        });
        // SAFETY: Empty NSDictionary has the declared UIKit key/value generic types.
        // App and URL remain retained for the call on the proven main thread; UIKit
        // copies the completion block for asynchronous use. It captures no objects.
        unsafe {
            app.openURL_options_completionHandler(&url, &NSDictionary::new(), Some(&completion));
        }
        // This synchronous service reports accepted dispatch; UIKit reports the final
        // asynchronous outcome through the completion handler above.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_preserves_straight_alpha_pixels() {
        let pixels = [255, 0, 12, 0, 4, 80, 200, 127, 1, 2, 3, 255, 0, 0, 0, 40];
        let png = encode_png(2, 2, &pixels, &limits()).unwrap();
        assert_eq!(decode_png(&png, &limits()).unwrap(), (2, 2, pixels.to_vec()));
    }
    #[test]
    fn sixteen_bit_gray_png_converts_at_rgba8_service_boundary() {
        let image = Image::from_u16(2, 1, ChannelLayout::Gray, &[0, u16::MAX]).unwrap();
        let png = photocraft_codecs::encode(&image, Format::Png, &EncodeOptions::default()).unwrap();
        assert_eq!(decode_png(&png, &limits()).unwrap(), (2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]));
    }
    #[test]
    fn rejects_empty_mismatched_and_overflow_dimensions() {
        assert!(encode_png(0, 1, &[], &limits()).is_err());
        assert!(encode_png(1, 1, &[0; 3], &limits()).is_err());
        assert!(rgba_len(u32::MAX, u32::MAX, &Limits::none()).is_err());
    }
    #[test]
    fn malformed_nonimage_and_truncated_inputs_fail() {
        for bytes in [&b"not an image"[..], &b"\x89PNG\r\n\x1a\n"[..], &[][..]] {
            assert!(decode_png(bytes, &limits()).is_err());
        }
    }
    #[test]
    fn custom_bound_checks_encoded_input_and_decoded_dimensions() {
        let small = Limits { max_width: 1, max_height: 1, max_pixels: 1, max_alloc: 1024 };
        assert!(encode_png(2, 1, &[0; 8], &small).is_err());
        let png = encode_png(2, 1, &[0; 8], &limits()).unwrap();
        assert!(decode_png(&png, &small).is_err());
        assert!(decode_png(&png, &Limits { max_alloc: 1, ..small }).is_err());
    }
}
