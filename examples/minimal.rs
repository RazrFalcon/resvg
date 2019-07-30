use resvg::prelude::*;

fn main() {

    println!("running minimal");
    
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = resvg::Options::default();
    opt.usvg.path = Some(args[1].clone().into());

    let rtree = usvg::Tree::from_file(&args[1], &opt.usvg).unwrap();
    let backend = resvg::default_backend();
    let img = backend.render_to_image(&rtree, &opt).unwrap();
    let result = img.save(std::path::Path::new(&args[2]));
    print!("result {}", result);
}
