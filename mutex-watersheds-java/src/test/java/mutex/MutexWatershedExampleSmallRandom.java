package mutex;

import com.sun.javafx.application.ParametersImpl;
import javafx.stage.Stage;
import net.imglib2.img.array.ArrayCursor;
import net.imglib2.img.array.ArrayImg;
import net.imglib2.img.array.ArrayImgs;
import net.imglib2.img.basictypeaccess.array.DoubleArray;
import net.imglib2.img.basictypeaccess.array.LongArray;
import net.imglib2.type.numeric.integer.UnsignedLongType;
import net.imglib2.type.numeric.real.DoubleType;
import net.imglib2.view.Views;
import org.janelia.saalfeldlab.paintera.Paintera;

import java.util.Arrays;
import java.util.Random;

import javafx.application.Application;

public class MutexWatershedExampleSmallRandom {

    public static void main(String... args) {
        Application.launch(MutexWatershedExampleSmallRandom.Impl.class, args);
    }

    public static class Impl extends Application {

        @Override
        public void start(Stage primaryStage) throws Exception {
            final Paintera paintera = new Paintera();
            ParametersImpl.registerParameters(paintera, getParameters());
            paintera.start(primaryStage);

            final ArrayImg<DoubleType, DoubleArray> data = ArrayImgs.doubles(4, 3, 2);
            final Random rng = new Random(1L);
            data.forEach(t -> t.setReal(2*rng.nextDouble() - 1.0));
            Views.hyperSlice(Views.hyperSlice(data, 2, 0L), 0, 0L).forEach(t -> t.setReal(Double.NaN));
            Views.hyperSlice(Views.hyperSlice(data, 2, 1L), 1, 0L).forEach(t -> t.setReal(Double.NaN));
            final int[] assignments = MutexWatershed.mutexWatershed(
                    Views.collapseReal(data),
                    new long[][]{{-1, 0}, {0, -1}});
            for (int i = 0; i < assignments.length; ++i)
                System.out.println(i + " -> " + assignments[i]);
            final ArrayImg<UnsignedLongType, LongArray> labels = ArrayImgs.unsignedLongs(4, 3, 1);
            final ArrayCursor<UnsignedLongType> c = labels.cursor();
            for (int i = 0; c.hasNext(); ++i) {
                c.next().setInteger(assignments[i]);
            }


            final double[] resolution = new double[] {1.0, 1.0, 100.0};
            final double[] offset = new double[] {0.0, 0.0, 0.0};

            // TODO no convenient way to add channel source; add two raw sources instead for affinities
            paintera.getMainWindow().getBaseView().addSingleScaleRawSource(
                    Views.addDimension(Views.hyperSlice(data, 2, 0L), 0L, 0L),
                    resolution, offset,
                    -1.0, 1.0,
                    "affinities x"
            );

            paintera.getMainWindow().getBaseView().addSingleScaleRawSource(
                    Views.addDimension(Views.hyperSlice(data, 2, 1L), 0L, 0L),
                    resolution, offset,
                    -1.0, 1.0,
                    "affinities y"
            );

            paintera.getMainWindow().getBaseView().addSingleScaleLabelSource(
                    labels,
                    resolution,
                    offset,
                    Arrays.stream(assignments).max().orElse(0),
                    "mutex ws"
            );

        }
    }
}