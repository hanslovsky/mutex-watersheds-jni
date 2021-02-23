// This is the interface to the JVM that we'll call the majority of our
// methods on.
use jni::JNIEnv;

// use jni::descriptors::Desc;

// These objects are what you should use as arguments to your native
// function. They carry extra lifetime information to prevent them escaping
// this context and getting used after being GC'd.
use jni::objects::{JClass, JObject, JValue, ReleaseMode};

use jni::signature::{JavaType, Primitive};

// This is just a pointer. We'll be returning it from our function. We
// can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jarray, jintArray, jlongArray, jobject};

use r_mutex_watersheds::mutex;

// This keeps Rust from "mangling" the name and making it unique for this
// crate.
#[no_mangle]
pub extern "system" fn Java_mutex_MutexWatershed_mutexWatershed(
    env: JNIEnv,
// This is the class that owns our static method. It's not going to be used,
// but still must be present to match the expected signature of a static
// native method.
    class: JClass,
    affinities: jobject,
    assignments: jintArray,
    offset_strides: jlongArray) {

    println!("LOL WAS LOS?");

    let num_offsets = env.get_array_length(offset_strides).unwrap();
    let num_dimensions = env.call_method(affinities, "numDimensions", "()I", &[]).unwrap().i().unwrap();
    let mut offsets: Vec<i64> = vec![0; num_offsets as usize];
    env.get_long_array_region(offset_strides, 0, &mut offsets[..]);
    let offsets = offsets;

    let views = env
        .find_class("net/imglib2/view/Views")
        .expect("Failed to load Java class net.imglib2.view.Views");

    let flatIterable = env.call_static_method(
        views,
        "flatIterable",
        "(Lnet/imglib2/RandomAccessibleInterval;)Lnet/imglib2/IterableInterval;",
        &[JValue::from(affinities)]).unwrap().l().unwrap();

    let cursor = env.call_method(flatIterable, "cursor", "()Lnet/imglib2/Cursor;", &[]).unwrap().l().unwrap();

    let has_next_method_id = env.get_method_id("net/imglib2/Cursor", "hasNext", "()Z").unwrap();
    let next_method_id = env.get_method_id("net/imglib2/Cursor", "next", "()Ljava/lang/Object;").unwrap();
    let composite_get_method_id = env.get_method_id("net/imglib2/view/composite/Composite", "get", "(J)Ljava/lang/Object;").unwrap();
    let real_double_method_id = env.get_method_id("net/imglib2/type/numeric/RealType", "getRealDouble", "()D").unwrap();
    // let composite = JavaType::Object(String::from("net/imglib2/view/composite/Composite"));

    let mut edges: Vec<(u32, u32, f64, bool)> = Vec::new();
    let mut current_position: i64 = 0;
    while env.call_method_unchecked(cursor, has_next_method_id, JavaType::Primitive(Primitive::Boolean), &[]).unwrap().z().unwrap() {
        // let c = env.call_method(cursor, "next", "()Ljava/lang/Object;", &[]).unwrap().l().unwrap();
        // Is it more efficient to create composite object outside of loop? I tried but it did not work.
        let c = env.call_method_unchecked(cursor, next_method_id, JavaType::Object(String::from("net/imglib2/view/composite/Composite")), &[]).unwrap().l().unwrap();
        for index in 0..num_offsets {
            let rt = env.call_method_unchecked(c, composite_get_method_id, JavaType::Object(String::from("net/imglib2/type/numeric/RealType")), &[JValue::Long(index as i64)]).unwrap().l().unwrap();
            let w = env.call_method_unchecked(rt, real_double_method_id, JavaType::Primitive(Primitive::Double), &[]).unwrap().d().unwrap();
            if w.is_nan() { continue };
            let o = offsets[index as usize];
            let (from, to) = if o < 0 { (current_position + o, current_position) } else { (current_position, current_position + o) };
            edges.push((from as u32, to as u32, w.abs(), w < 0.0));
            // println!("cp={} index={} w={} o={}", current_position, index, w, offsets[index as usize]);
        }
        current_position += 1;
    }

    let intervals = env.find_class("net/imglib2/util/Intervals").expect("Failed to load Java class net.imglib2.util.Intervals");
    let num_labels = env.call_static_method(intervals, "numElements", "(Lnet/imglib2/Dimensions;)J", &[JValue::from(affinities)]).unwrap().j().unwrap() as u32;

    println!("edges={:?}", edges);

    let uf = mutex::compute_mutex_watershed_clustering(num_labels as usize, &edges);
    let assignment_vec = (0..num_labels).map(|i| uf.find(i) as i32).collect::<Vec<i32>>();
    env.set_int_array_region(assignments, 0, &assignment_vec[..]);

    println!("LUL WAS LOS? {}", num_offsets);

}

// fn hasNext(env: JNIEnv, cursor: JObject) -> bool {
//     let result = env.call_method(cursor, "hasNext", "()Z", &[]);
//     if result.is_err() {
//         let exc = result.unwrap_err();
//         // println!("desc {}", exc.kind());//exc.description().to_string());
//         // println!("{}", exc.backtrace().unwrap());
//         format!("WAS DA LOS? {}", exc);
//         return false; // panic!("blub");
//     } else {
//         return result.unwrap().z().unwrap();
//     }
// }

