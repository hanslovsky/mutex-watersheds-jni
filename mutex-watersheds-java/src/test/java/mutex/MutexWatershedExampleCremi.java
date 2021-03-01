package mutex;

import com.sun.javafx.application.ParametersImpl;
import javafx.application.Application;
import javafx.application.Platform;
import javafx.stage.Stage;
import net.imglib2.Cursor;
import net.imglib2.FinalInterval;
import net.imglib2.Interval;
import net.imglib2.RandomAccessibleInterval;
import net.imglib2.cache.img.CachedCellImg;
import net.imglib2.img.array.ArrayCursor;
import net.imglib2.img.array.ArrayImg;
import net.imglib2.img.array.ArrayImgs;
import net.imglib2.img.basictypeaccess.array.DoubleArray;
import net.imglib2.img.basictypeaccess.array.LongArray;
import net.imglib2.loops.LoopBuilder;
import net.imglib2.type.numeric.integer.UnsignedByteType;
import net.imglib2.type.numeric.integer.UnsignedLongType;
import net.imglib2.type.numeric.real.DoubleType;
import net.imglib2.util.Intervals;
import net.imglib2.view.IntervalView;
import net.imglib2.view.Views;
import org.janelia.saalfeldlab.n5.hdf5.N5HDF5Reader;
import org.janelia.saalfeldlab.n5.imglib2.N5Utils;
import org.janelia.saalfeldlab.paintera.Paintera;

import java.util.Arrays;
import java.util.Random;

public class MutexWatershedExampleCremi {

    public static void main(String... args) {
        Application.launch(MutexWatershedExampleCremi.Impl.class, args);
    }

    public static class Impl extends Application {

        @Override
        public void start(Stage primaryStage) throws Exception {

            final Interval cutOut = Intervals.createMinMax(400, 400, 45, 700, 700, 65);
//            final Interval cutOut = new FinalInterval(600, 600, 60);

            final N5HDF5Reader hdf5 = new N5HDF5Reader("/home/zottel/Downloads/sample_B_20160501.hdf", true, 32, 32, 3);
            final RandomAccessibleInterval<UnsignedByteType> raw = Views.zeroMin(Views.interval(N5Utils.<UnsignedByteType>open(hdf5, "volumes/raw"), cutOut));
            final RandomAccessibleInterval<UnsignedLongType> groundTruth = Views.zeroMin(Views.interval(N5Utils.<UnsignedLongType>open(hdf5, "volumes/labels/neuron_ids"), cutOut));

            final Paintera paintera = new Paintera();
            ParametersImpl.registerParameters(paintera, getParameters());
            paintera.start(primaryStage);


            new Thread(() -> {
                final double[] resolution = new double[]{1.0, 1.0, 10.0};
                final double[] offset = new double[]{0.0, 0.0, 0.0};

                final Random rng = new Random(1L);
                final double flipProbability = 0.0;//1e-5;

                System.out.println("Preparing data");
                final ArrayImg<DoubleType, DoubleArray> data = ArrayImgs.doubles(raw.dimension(0), raw.dimension(1), raw.dimension(2), 3);
                for (int d = 0; d < 3; ++d) {
                    Views.hyperSlice(data, d, 0L).forEach(vx -> vx.setReal(Double.NaN));
                    final long[] min1 = Intervals.minAsLongArray(raw);
                    final long[] min2 = Intervals.minAsLongArray(raw);
                    final long[] max1 = Intervals.maxAsLongArray(raw);
                    final long[] max2 = Intervals.maxAsLongArray(raw);
                    max1[d] -= 1;
                    min2[d] += 1;
                    LoopBuilder
                            .setImages(
                                    Views.interval(groundTruth, min1, max1),
                                    Views.interval(groundTruth, min2, max2),
                                    Views.interval(Views.hyperSlice(data, 3, d), min2, max2))
                            .forEachPixel((gt1, gt2, aff) -> aff.setReal((gt1.valueEquals(gt2) ? 1.0 : -1.0) * (rng.nextDouble() < flipProbability ? -1.0 : 1.0)));
                }

                System.out.println("Running mutex watershed");
                final int[] assignments = MutexWatershed.mutexWatershed(
                        Views.collapseReal(data),
                        new long[][]{{-1, 0, 0}, {0, -1, 0}, {0, 0, -1}});
                final ArrayImg<UnsignedLongType, LongArray> labels = ArrayImgs.unsignedLongs(Intervals.dimensionsAsLongArray(raw));
                final ArrayCursor<UnsignedLongType> c = labels.cursor();
                System.out.println("Relabeling: " + labels);
                for (int i = 0; c.hasNext(); ++i)
                    c.next().setInteger(assignments[i]);

                System.out.print("Showing data");
                paintera
                        .getMainWindow()
                        .getBaseView()
                        .addSingleScaleRawSource(raw, resolution, offset, 0, 255, "raw");

                paintera.getMainWindow().getBaseView().addSingleScaleLabelSource(
                        labels,
                        resolution,
                        offset,
                        Arrays.stream(assignments).max().orElse(0),
                        "mutex ws"
                );
            }).start();

        }
    }
}