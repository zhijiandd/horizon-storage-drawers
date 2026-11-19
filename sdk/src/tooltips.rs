use crate::{compat::Stack, engine as sd, items};
use mod_api::*;

fn ring_grid(count: usize) -> (usize, usize) {
    let columns = ((count + 1) as f64).sqrt().ceil().max(2.) as usize;
    (columns, (count + 1).div_ceil(columns))
}

pub fn size(host: &dyn ClientHost, stack: &ItemStack) -> Option<[f32; 2]> {
    let stack = Stack::from_api(host, stack)?;
    if items::detached(&stack).is_some() { return Some([128., 28.]); }
    if !items::kind(&stack.item).starts_with("keyring") { return None; }
    let (columns, rows) = ring_grid(items::ring_items(&stack).len());
    Some([(columns * 18 + 2) as f32, (rows * 20 + 6) as f32])
}

pub fn draw(host: &dyn ClientHost, event: &mut TooltipComponentDrawEvent<'_>) {
    let Some(stack) = Stack::from_api(host, event.stack) else { return; };
    let [x, y] = event.origin;
    if let Some((stock, limit, _)) = items::detached(&stack) {
        let check_limit = sd::config::boolean("detached_force_capacity_check", false);
        let texture_y = if check_limit { 0. } else { 24. };
        event.draw.sprite("storagedrawers:gui/detached_tooltip", [0., texture_y, 128., 24.], [x, y, 128., 24.], None);
        let count = stock.as_ref().map_or(0, |stock| stock.count);
        if let Some(item) = stock.and_then(|stock| stock.to_api(host)) {
            event.draw.item_stack(&item, [x + 4., y + 4.], 16.);
        }
        event.draw.text(&count.to_string(), [x + 22., y + 8.], 0xff808080, false);
        if check_limit {
            event.draw.text(&limit.to_string(), [x + 83., y + 8.], 0xff808080, false);
        }
    } else if items::kind(&stack.item).starts_with("keyring") {
        let keys = items::ring_items(&stack);
        let (columns, rows) = ring_grid(keys.len());
        event.draw.fill([x, y, (columns * 18 + 2) as f32, (rows * 20 + 2) as f32], 0xff30243a);
        for index in 0..columns * rows {
            let left = x + 1. + (index % columns * 18) as f32;
            let top = y + 1. + (index / columns * 20) as f32;
            event.draw.sprite("gui/sprites/container/bundle/slot_background", [0., 0., 18., 20.],
                [left, top, 18., 20.], None);
            if let Some(key) = keys.get(index).and_then(|key| key.to_api(host)) {
                event.draw.item_stack(&key, [left + 1., top + 1.], 16.);
            }
        }
    }
}
