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
        area.x.saturating_add(width),
        main_y.saturating_add(main_height),
    );

    if main_area.width < 10 || main_area.height < 4 {
        return;
    }

    let panel_height = 4u16;
    let mid_x = area.x.saturating_add(width / 3);
    let mid_x2 = area.x.saturating_add((width * 2) / 3);

    let cpu_area = Rect::new(
        area.x,
        main_area.y,
        mid_x,
        main_area.y.saturating_add(panel_height),
    );
    let gpu_area = Rect::new(
        mid_x,
        main_area.y,
        mid_x2,
        main_area.y.saturating_add(panel_height),
    );
    let mem_area = Rect::new(
        mid_x2,
        main_area.y,
        area.x.saturating_add(width),
        main_area.y.saturating_add(panel_height),
    );

    render_cpu(
        f,
        cpu_area,
        &app.data.cpu,
        crate::ui::DisplayMode::Compact,
        app.cpu_history.get(),
    );
    render_gpu(
        f,
        gpu_area,
        &app.data.gpu,
        crate::ui::DisplayMode::Compact,
        app.gpu_history.get(),
    );
    render_memory(f, mem_area, &app.data.memory, app.show_swap);

    let network_disk_y = main_area.y.saturating_add(panel_height).saturating_add(1);
    let panel_height2 = main_area
        .y
        .saturating_add(main_height)
        .saturating_sub(network_disk_y)
        .saturating_sub(1);

    if panel_height2 > 0 {
        let net_area = Rect::new(
            area.x,
            network_disk_y,
            mid_x,
            network_disk_y.saturating_add(panel_height2),
        );
        let disk_area = Rect::new(
            mid_x,
            network_disk_y,
            area.x.saturating_add(width),
            network_disk_y.saturating_add(panel_height2),
        );

        render_network(f, net_area, &app.data.network);
        render_disk(f, disk_area, &app.data.disks);
    }
}
