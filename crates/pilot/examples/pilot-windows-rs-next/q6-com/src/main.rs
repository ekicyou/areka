mod renderer;
mod shiori;

fn main() {
    shiori::run();
    let lines = renderer::collect_underlines().expect("q6 renderer");
    for u in &lines {
        println!(
            "q6 DrawUnderline x={} y={} width={} thickness={} orientation={} renderer1={}",
            u.x, u.y, u.width, u.thickness, u.orientation, u.renderer1
        );
    }
    println!("q6 underlines={}", lines.len());
}
