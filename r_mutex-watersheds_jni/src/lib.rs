extern crate pretty_env_logger;

// This is the interface to the JVM that we'll call the majority of our
// methods on.
use jni::JNIEnv;

// use jni::descriptors::Desc;

// These objects are what you should use as arguments to your native
// function. They carry extra lifetime information to prevent them escaping
// this context and getting used after being GC'd.
use jni::objects::{JClass, JValue};

use jni::signature::{JavaType, Primitive};

// This is just a pointer. We'll be returning it from our function. We
// can't return one of the objects with lifetime information because the
// lifetime checker won't let us.
use jni::sys::{jdoubleArray, jintArray, jlongArray, jobject};

use log::{debug, trace};
use pretty_env_logger as _log;

use mutex_watersheds::mutex;

// NOTE Iterating over RAI in Rust JNI is extremely slow and generating the graph takes a long time.
//      It is faster to copy from RAI into double[], then copy from double[] to Vec with a single call.
//      Java_mutex_MutexWatershed_mutexWatershed is preferred over
//      Java_mutex_MutexWatershed_mutexWatershedFromRAI


// This keeps Rust from "mangling" the name and making it unique for this
// crate.
#[no_mangle]
pub extern "system" fn Java_mutex_MutexWatershed_mutexWatershed(
    env: JNIEnv,
// This is the class that owns our static method. It's not going to be used,
// but still must be present to match the expected signature of a static
// native method.
    _class: JClass,
    affinities: jdoubleArray,
    assignments: jintArray,
    offset_strides: jlongArray) {

    let _ = _log::try_init();

    debug!("LOL WAS LOS?");

    let num_offsets = env.get_array_length(offset_strides).unwrap();
    let mut offsets: Vec<i64> = vec![0; num_offsets as usize];
    let _ = env.get_long_array_region(offset_strides, 0, &mut offsets[..]);
    let offsets = offsets;
    let mut affinities_copy: Vec<f64> = vec![0.0; env.get_array_length(affinities).unwrap() as usize];
    let _ = env.get_double_array_region(affinities, 0, &mut affinities_copy[..]);
    let affinities = affinities_copy;
    let num_labels = affinities.len() / num_offsets as usize;


    debug!("Building graph");
    let mut edges: Vec<(u32, u32, f64, bool)> = Vec::new();
    let mut current_position: i64 = 0;
    let mut index = 0;
    while index < affinities.len() {
        if current_position % 10000 == 0 { trace!("current_position={}/{}", current_position, num_labels); }
        for offset_index in 0..num_offsets {
            let w = affinities[index];
            if !w.is_nan() {
                let offset = offsets[offset_index as usize];
                let (from, to) = if offset < 0 {
                    (current_position + offset, current_position)
                } else {
                    (current_position, current_position + offset)
                };
                edges.push((from as u32, to as u32, w.abs(), w < 0.0));
            }
            index += 1;
        }
        current_position += 1;
    }

    let uf = mutex::compute_mutex_watershed_clustering(num_labels as usize, &edges);
    let assignment_vec = (0u32..num_labels as u32).map(|i| uf.find(i) as i32).collect::<Vec<i32>>();
    let _ = env.set_int_array_region(assignments, 0, &assignment_vec[..]);

    debug!("LUL WAS LOS? {}", num_offsets);

}

// This keeps Rust from "mangling" the name and making it unique for this
// crate.
#[no_mangle]
pub extern "system" fn Java_mutex_MutexWatershed_mutexWatershedFromRAI(
    env: JNIEnv,
// This is the class that owns our static method. It's not going to be used,
// but still must be present to match the expected signature of a static
// native method.
    _class: JClass,
    affinities: jobject,
    assignments: jintArray,
    offset_strides: jlongArray) {

    let _ = _log::try_init();

    debug!("LOL WAS LOS?");

    let num_offsets = env.get_array_length(offset_strides).unwrap();
    let mut offsets: Vec<i64> = vec![0; num_offsets as usize];
    let _ = env.get_long_array_region(offset_strides, 0, &mut offsets[..]);
    let offsets = offsets;

    let views = env
        .find_class("net/imglib2/view/Views")
        .expect("Failed to load Java class net.imglib2.view.Views");

    let flat_iterable = env.call_static_method(
        views,
        "flatIterable",
        "(Lnet/imglib2/RandomAccessibleInterval;)Lnet/imglib2/IterableInterval;",
        &[JValue::from(affinities)]).unwrap().l().unwrap();

    let cursor = env.call_method(flat_iterable, "cursor", "()Lnet/imglib2/Cursor;", &[]).unwrap().l().unwrap();

    let has_next_method_id = env.get_method_id("net/imglib2/Cursor", "hasNext", "()Z").unwrap();
    let next_method_id = env.get_method_id("net/imglib2/Cursor", "next", "()Ljava/lang/Object;").unwrap();
    let composite_get_method_id = env.get_method_id("net/imglib2/view/composite/Composite", "get", "(J)Ljava/lang/Object;").unwrap();
    let real_double_method_id = env.get_method_id("net/imglib2/type/numeric/RealType", "getRealDouble", "()D").unwrap();
    let composite = JavaType::Object(String::from("net/imglib2/view/composite/Composite"));
    let real_type = JavaType::Object(String::from("net/imglib2/type/numeric/RealType"));
    let primitive_double = JavaType::Primitive(Primitive::Double);
    let primitive_boolean = JavaType::Primitive(Primitive::Boolean);

    debug!("Building graph");
    let mut edges: Vec<(u32, u32, f64, bool)> = Vec::new();
    let mut current_position: i64 = 0;
    let intervals = env.find_class("net/imglib2/util/Intervals").expect("Failed to load Java class net.imglib2.util.Intervals");
    let num_labels = env.call_static_method(intervals, "numElements", "(Lnet/imglib2/Dimensions;)J", &[JValue::from(affinities)]).unwrap().j().unwrap() as u32;
    while env.call_method_unchecked(cursor, has_next_method_id, primitive_boolean.clone(), &[]).unwrap().z().unwrap() {
        // let c = env.call_method(cursor, "next", "()Ljava/lang/Object;", &[]).unwrap().l().unwrap();
        // Is it more efficient to create composite object outside of loop? I tried but it did not work.
        let c = env.call_method_unchecked(cursor, next_method_id, composite.clone(), &[]).unwrap().l().unwrap();
        if current_position % 10000 == 0 { trace!("current_position={}/{}", current_position, num_labels); }
        for index in 0..num_offsets {
            let rt = env.call_method_unchecked(c, composite_get_method_id, real_type.clone(), &[JValue::Long(index as i64)]).unwrap().l().unwrap();
            let w = env.call_method_unchecked(rt, real_double_method_id, primitive_double.clone(), &[]).unwrap().d().unwrap();
            if w.is_nan() { continue };
            let o = offsets[index as usize];
            let (from, to) = if o < 0 { (current_position + o, current_position) } else { (current_position, current_position + o) };
            edges.push((from as u32, to as u32, w.abs(), w < 0.0));
            // trace!("cp={} index={} w={} o={}", current_position, index, w, offsets[index as usize]);
        }
        current_position += 1;
    }


    let uf = mutex::compute_mutex_watershed_clustering(num_labels as usize, &edges);
    let assignment_vec = (0..num_labels).map(|i| uf.find(i) as i32).collect::<Vec<i32>>();
    let _ = env.set_int_array_region(assignments, 0, &assignment_vec[..]);

    debug!("LUL WAS LOS? {}", num_offsets);

}


