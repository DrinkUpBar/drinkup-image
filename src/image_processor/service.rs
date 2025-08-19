use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use tracing::info;

use crate::image_processor::model::AppError;

/// 图片处理服务
pub struct ImageProcessService;

impl ImageProcessService {
    pub fn new() -> Self {
        Self
    }

    /// 下载图片
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, AppError> {
        info!("从URL下载图片: {}", url);
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// 解码Base64图片数据
    pub fn decode_base64_image(&self, data: &str) -> Result<Vec<u8>, AppError> {
        info!("解码Base64图片数据");
        Ok(general_purpose::STANDARD.decode(data)?)
    }

    /// 处理图像数据
    pub async fn process_image_data(&self, image_data: Vec<u8>) -> Result<DynamicImage, AppError> {
        // 在异步上下文中处理图像
        tokio::task::spawn_blocking(move || -> Result<DynamicImage, AppError> {
            // 解码图像
            let mut image = image::load_from_memory(&image_data)?;

            // 移除背景
            let bg_remover = BackgroundRemover::new();
            image = bg_remover.remove_background(image)?;

            Ok(image)
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
    }

    /// 将图像转换为指定格式的Base64字符串
    pub fn image_to_base64(&self, image: DynamicImage, format: &str) -> Result<String, AppError> {
        let mut output_bytes = Vec::new();
        let image_format = match format {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::WebP,
            _ => ImageFormat::Png,
        };

        image.write_to(&mut std::io::Cursor::new(&mut output_bytes), image_format)?;
        Ok(general_purpose::STANDARD.encode(&output_bytes))
    }
}

impl Default for ImageProcessService {
    fn default() -> Self {
        Self::new()
    }
}

/// 背景移除器
pub struct BackgroundRemover {
    /// 色彩容差
    color_tolerance: f32,
    /// 边缘模糊程度
    edge_blur: u32,
}

impl BackgroundRemover {
    /// 创建新的背景移除器
    pub fn new() -> Self {
        Self {
            color_tolerance: 30.0,
            edge_blur: 2,
        }
    }

    /// 移除背景
    pub fn remove_background(
        &self,
        image: DynamicImage,
    ) -> Result<DynamicImage, image::ImageError> {
        let rgba_image = image.to_rgba8();
        let (_width, _height) = rgba_image.dimensions();

        // 检测背景色（使用四个角落的平均色）
        let background_color = self.detect_background_color(&rgba_image);

        // 创建蒙版
        let mask = self.create_mask(&rgba_image, background_color);

        // 应用蒙版
        let result = self.apply_mask(&rgba_image, &mask);

        Ok(DynamicImage::ImageRgba8(result))
    }

    /// 检测背景色
    fn detect_background_color(&self, image: &RgbaImage) -> Rgba<u8> {
        let (width, height) = image.dimensions();
        let mut corner_colors = Vec::new();

        // 采样四个角落的颜色
        let sample_size = 5.min(width / 4).min(height / 4);

        // 左上角
        for x in 0..sample_size {
            for y in 0..sample_size {
                corner_colors.push(*image.get_pixel(x, y));
            }
        }

        // 右上角
        for x in (width - sample_size)..width {
            for y in 0..sample_size {
                corner_colors.push(*image.get_pixel(x, y));
            }
        }

        // 左下角
        for x in 0..sample_size {
            for y in (height - sample_size)..height {
                corner_colors.push(*image.get_pixel(x, y));
            }
        }

        // 右下角
        for x in (width - sample_size)..width {
            for y in (height - sample_size)..height {
                corner_colors.push(*image.get_pixel(x, y));
            }
        }

        // 计算平均颜色
        let total_colors = corner_colors.len() as f32;
        let avg_r = corner_colors.iter().map(|c| c[0] as f32).sum::<f32>() / total_colors;
        let avg_g = corner_colors.iter().map(|c| c[1] as f32).sum::<f32>() / total_colors;
        let avg_b = corner_colors.iter().map(|c| c[2] as f32).sum::<f32>() / total_colors;

        Rgba([avg_r as u8, avg_g as u8, avg_b as u8, 255])
    }

    /// 创建蒙版
    fn create_mask(&self, image: &RgbaImage, background_color: Rgba<u8>) -> Vec<Vec<bool>> {
        let (width, height) = image.dimensions();
        let mut mask = vec![vec![true; width as usize]; height as usize];

        // 基于颜色相似度创建初始蒙版
        for y in 0..height {
            for x in 0..width {
                let pixel = *image.get_pixel(x, y);
                if self.color_distance(&pixel, &background_color) <= self.color_tolerance {
                    mask[y as usize][x as usize] = false;
                }
            }
        }

        // 使用泛洪填充去除连接的背景区域
        self.flood_fill_background(&mut mask, width as usize, height as usize);

        mask
    }

    /// 泛洪填充背景
    fn flood_fill_background(&self, mask: &mut [Vec<bool>], width: usize, height: usize) {
        let mut visited = vec![vec![false; width]; height];

        // 从四个边缘开始泛洪填充
        let edges = [
            // 上边缘
            (0..width).map(|x| (x, 0)).collect::<Vec<_>>(),
            // 下边缘
            (0..width).map(|x| (x, height - 1)).collect::<Vec<_>>(),
            // 左边缘
            (0..height).map(|y| (0, y)).collect::<Vec<_>>(),
            // 右边缘
            (0..height).map(|y| (width - 1, y)).collect::<Vec<_>>(),
        ]
        .concat();

        for (x, y) in edges {
            if !visited[y][x] && !mask[y][x] {
                self.flood_fill_iterative(mask, &mut visited, x, y, width, height);
            }
        }
    }

    /// 迭代泛洪填充（避免栈溢出）
    fn flood_fill_iterative(
        &self,
        mask: &mut [Vec<bool>],
        visited: &mut [Vec<bool>],
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
    ) {
        let mut stack = Vec::new();
        stack.push((start_x, start_y));

        while let Some((x, y)) = stack.pop() {
            if visited[y][x] || mask[y][x] {
                continue;
            }

            visited[y][x] = true;
            mask[y][x] = false; // 标记为背景

            // 检查四个方向
            let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
            for (dx, dy) in directions.iter() {
                let new_x = x as i32 + dx;
                let new_y = y as i32 + dy;

                if new_x >= 0 && new_x < width as i32 && new_y >= 0 && new_y < height as i32 {
                    let new_x = new_x as usize;
                    let new_y = new_y as usize;

                    if !visited[new_y][new_x] && !mask[new_y][new_x] {
                        stack.push((new_x, new_y));
                    }
                }
            }
        }
    }

    /// 应用蒙版到图像
    fn apply_mask(&self, image: &RgbaImage, mask: &[Vec<bool>]) -> RgbaImage {
        let (width, height) = image.dimensions();
        let mut result = RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let mut pixel = *image.get_pixel(x, y);

                if !mask[y as usize][x as usize] {
                    // 背景像素，设置为透明
                    pixel[3] = 0;
                } else {
                    // 前景像素，进行边缘羽化
                    let alpha = self.calculate_edge_alpha(
                        mask,
                        x as usize,
                        y as usize,
                        width as usize,
                        height as usize,
                    );
                    pixel[3] = ((pixel[3] as f32) * alpha) as u8;
                }

                result.put_pixel(x, y, pixel);
            }
        }

        result
    }

    /// 计算边缘alpha值进行羽化
    fn calculate_edge_alpha(
        &self,
        mask: &[Vec<bool>],
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> f32 {
        if !mask[y][x] {
            return 0.0;
        }

        let blur_radius = self.edge_blur as i32;
        let mut background_count = 0;
        let mut total_count = 0;

        for dy in -blur_radius..=blur_radius {
            for dx in -blur_radius..=blur_radius {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    total_count += 1;
                    if !mask[ny as usize][nx as usize] {
                        background_count += 1;
                    }
                }
            }
        }

        if total_count == 0 {
            return 1.0;
        }

        let background_ratio = background_count as f32 / total_count as f32;
        (1.0 - background_ratio).max(0.0)
    }

    /// 计算两个颜色之间的距离
    fn color_distance(&self, color1: &Rgba<u8>, color2: &Rgba<u8>) -> f32 {
        let dr = color1[0] as f32 - color2[0] as f32;
        let dg = color1[1] as f32 - color2[1] as f32;
        let db = color1[2] as f32 - color2[2] as f32;

        (dr * dr + dg * dg + db * db).sqrt()
    }
}

impl Default for BackgroundRemover {
    fn default() -> Self {
        Self::new()
    }
}
