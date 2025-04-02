use aojia::*;

fn main() {
    let aojia = AoJia::new_with_path(String::from("ARegJ64.dll"), String::from("AoJia64.dll")).unwrap();
    println!("插件版本：{}", aojia.VerS().unwrap());

    let ret = aojia.GetMachineCode().unwrap();
    println!("GetMachineCode ret: {}", ret);

    let mut v = String::new();
    let mut vn = String::new();
    let mut vbn = -1;
    let mut dir = String::new();

    let ret = aojia.GetOs(&mut v, &mut vn, &mut vbn, &mut dir, 0).unwrap();
    println!("GetOs ret: {}, v: {}, vn: {}, vbn: {}, dir: {}", ret, v, vn, vbn, dir);

    let mut ty = String::new();
    let mut id = String::new();

    let ret = aojia.GetCPU(&mut ty, &mut id).unwrap();
    println!("GetCPU ret: {}, ty: {}, id: {}", ret, ty, id);

}

