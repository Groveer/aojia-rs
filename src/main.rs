use aj_rs::ao_jia::*;

fn main() {
    let aojia = AoJia::new_with_path(String::from("ARegJ64.dll"), String::from("AoJia64.dll")).unwrap();
    println!("插件版本：{}", aojia.VerS().unwrap());

    let ret = aojia.SetPath("\\").unwrap();
    println!("SetPath ret: {}", ret);
}
