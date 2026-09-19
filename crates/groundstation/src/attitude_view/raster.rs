//! Software triangle rasteriser, vendored from the standalone 3D renderer.
//!
//! Writes `0RGB` pixels into a flat `Vec<u32>` framebuffer of fixed
//! [`WIDTH`](super::WIDTH) x [`HEIGHT`](super::HEIGHT). No `minifb`, no depth
//! buffer: visibility is by back-face culling plus a painter's-algorithm depth
//! sort done by the caller.

use std::mem;

use super::{HEIGHT, WIDTH};

#[derive(Debug)]
pub struct Tri {
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

fn draw_horiz_line(buffer: &mut [u32], x1: u32, x2: u32, y: u32, colour: u32) {
    if (y as usize) >= HEIGHT {
        return;
    }
    let y_offset = (HEIGHT - (y as usize) - 1) * WIDTH;
    let (lo, hi) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
    for i in (lo as usize)..=(hi as usize) {
        if i < WIDTH {
            buffer[y_offset + i] = colour;
        }
    }
}

/// Order three points by decreasing y (`pmax.y >= pmid.y >= pmin.y`).
fn sort_points_by_y(p1: Point, p2: Point, p3: Point) -> (Point, Point, Point) {
    let mut pmax = p1;
    let mut pmid = p2;
    let mut pmin = p3;

    if pmid.y > pmax.y {
        mem::swap(&mut pmax, &mut pmid);
    }
    if pmin.y > pmax.y {
        mem::swap(&mut pmax, &mut pmin);
    }
    if pmin.y > pmid.y {
        mem::swap(&mut pmid, &mut pmin);
    }
    (pmax, pmid, pmin)
}

/// Fill a triangle by splitting it at the middle vertex's y into a
/// flat-bottom and a flat-top half, then scanning horizontal spans.
pub fn draw_filled_triangle(buffer: &mut [u32], tri: &Tri, colour: u32) {
    let sorted = sort_points_by_y(tri.p1, tri.p2, tri.p3);

    let p4y = sorted.1.y;
    let num = (sorted.0.y as f32) - (sorted.2.y as f32);
    let denom = (sorted.0.x as f32) - (sorted.2.x as f32);

    let p4x = if denom == 0. {
        sorted.0.x
    } else {
        let gradient = num / denom;
        let c = (sorted.0.y as f32) - gradient * (sorted.0.x as f32);
        (((p4y as f32) - c) / gradient).round() as u32
    };

    draw_flat_bottom_triangle(buffer, sorted.0, sorted.1.x, p4x, p4y, colour);
    draw_flat_top_triangle(buffer, sorted.2, sorted.1.x, p4x, p4y, colour);
}

fn draw_flat_bottom_triangle(
    buffer: &mut [u32],
    p1: Point,
    p2x: u32,
    p3x: u32,
    p23y: u32,
    colour: u32,
) {
    let inv_grad_p2_p1 = ((p2x as f32) - (p1.x as f32)) / ((p23y as f32) - (p1.y as f32));
    let inv_grad_p1_p3 = ((p1.x as f32) - (p3x as f32)) / ((p1.y as f32) - (p23y as f32));

    let mut from = p2x as f32;
    let mut to = p3x as f32;

    for y in p23y..p1.y {
        draw_horiz_line(buffer, from.round() as u32, to.round() as u32, y, colour);
        from += inv_grad_p2_p1;
        to += inv_grad_p1_p3;
    }
}

fn draw_flat_top_triangle(
    buffer: &mut [u32],
    p1: Point,
    p2x: u32,
    p3x: u32,
    p23y: u32,
    colour: u32,
) {
    let inv_grad_p2_p1 = ((p2x as f32) - (p1.x as f32)) / ((p23y as f32) - (p1.y as f32));
    let inv_grad_p1_p3 = ((p1.x as f32) - (p3x as f32)) / ((p1.y as f32) - (p23y as f32));

    let mut from = p1.x as f32;
    let mut to = p1.x as f32;

    for y in p1.y..p23y {
        draw_horiz_line(buffer, from.round() as u32, to.round() as u32, y, colour);
        from += inv_grad_p2_p1;
        to += inv_grad_p1_p3;
    }
}
