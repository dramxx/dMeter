fn render_compact_mode(f: &mut Frame, area: Rect, app: &App) {
    let width = area.width;
    let header_height = 1u16;
    let footer_height = 1u16;

    let main_y = area.y.saturating_add(header_height);
    let main_height = area
        .height
        .saturating_sub(header_height)
        .saturating_sub(footer_height);
    let main_area = Rect::new(
        area.x,
        main_y,
        width,
        main_height,
    );

    if main_area.width < 10 || main_area.height < 7 {
        return;
    }

    let panel_height = 7u16;
    let mid_x = area.x.saturating_add(width / 3);
    let mid_x2 = area.x.saturating_add((width * 2) / 3);

    let cpu_area = Rect::new(
        area.x,
        main_area.y,
        mid_x.saturating_sub(area.x),
        panel_height,
    );
    let gpu_area = Rect::new(
        mid_x,
        main_area.y,
        mid_x2.saturating_sub(mid_x),
        panel_height,
    );
    let mem_area = Rect::new(
        mid_x2,
        main_area.y,
        area.x.saturating_add(width).saturating_sub(mid_x2),
        panel_height,
    );

    render_cpu(
        f,
        cpu_area,
        &app.data.cpu,
        crate::ui::DisplayMode::Compact,
        app.cpu_history.get(),
    );
    render_gpu(f, gpu_area, &app.data.gpu);
}
}
