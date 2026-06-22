pub fn icon_product_mockup_prompt(
    scene: &str,
    scale_direction: &str,
    reference_count: usize,
) -> String {
    let reference_contract = if reference_count == 0 {
        r#"REFERENCE STATUS:
- No visual reference was supplied. Create a realistic premium product mockup based on the admin instruction."#
    } else if reference_count == 1 {
        r#"REFERENCE CONTRACT:
- Use Reference 1 as the product mockup template: wooden frame, warm light, QR module, button, phone, camera angle and background.
- Keep the product/mockup structure from Reference 1."#
    } else {
        r#"REFERENCE CONTRACT:
- Use Reference 1 as the product mockup template: wooden frame, warm light, QR module, button, phone, camera angle and background.
- Place Reference 2 inside the large blank/white artwork canvas inside the wooden frame.
- Also place a small cropped version of Reference 2 inside the circular avatar/profile placeholder on the phone screen.
- Do not place Reference 2 on the rest of the phone screen, QR module, button, table, background or any other area.
- Keep the phone screen layout, text blocks and audio controls from Reference 1; only fill the circular avatar/profile placeholder with Reference 2.
- Keep the QR module exactly as a QR module from Reference 1; do not replace it with Reference 2.
- Keep Reference 2 visually recognizable and preserve its composition, colors and details.
- Adjust only perspective, crop and lighting so it fits naturally inside the frame.
- Keep all product elements from Reference 1 unchanged."#
    };

    format!(
        r#"Generate ONE IMAGE ONLY. Do not write JSON, markdown, captions, explanations or article text.

Create a product mockup using the provided references.
Admin instruction: {scene}
Scale and style: {scale_direction}

{reference_contract}

PRODUCT DETAILS TO INCLUDE WHEN APPROPRIATE:
- carved or wooden standing icon frame
- warm edge light or soft product lighting
- QR module or QR plate near the icon
- optional phone/audio prayer presentation if it is requested by the admin instruction
- clean catalog composition, realistic object proportions, photorealistic material texture, high detail, 4K-quality look
- natural camera optics, realistic depth of field, believable shadows and reflections, no CGI/plastic look

Avoid adding readable new text, logos, watermarks, UI captions or marketing text.
Output: photorealistic premium 4K product photo."#,
        scene = scene,
        scale_direction = scale_direction,
        reference_contract = reference_contract,
    )
}

pub fn icon_product_mockup_fallback_prompt(scene: &str) -> String {
    format!(
        r#"Generate ONE IMAGE ONLY. Do not write JSON, markdown, captions, explanations or article text.

Replace only the artwork inside the wooden icon frame with Reference 2.
Keep everything else from Reference 1: frame, QR module, button, phone, lighting and composition.
Also place a small cropped version of Reference 2 inside the circular phone avatar/profile placeholder.
Do not alter the rest of the phone screen layout, text blocks or audio controls.
Do not redraw the inserted artwork.
Preserve Reference 2 composition and details.
Fit Reference 2 naturally into the frame with correct perspective and light.

Admin instruction: {scene}
Output: photorealistic premium product photo."#,
        scene = scene,
    )
}
