package com.zesp;

import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.os.Looper;
import android.util.Base64;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

/**
 * Run with `app_process` over adb. Prints one `<package> <base64 png>` line per launchable app.
 */
public final class IconDump {
    private static final int SIZE = 96;
    private static final int THREADS = 4;

    private IconDump() {
    }

    public static void main(String[] args) throws Exception {
        Looper.prepareMainLooper();
        PackageManager pm = systemContext().getPackageManager();
        PrintStream out = System.out;

        Intent launcher = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER);
        Set<String> seen = new HashSet<>();
        ExecutorService pool = Executors.newFixedThreadPool(THREADS);
        for (final ResolveInfo app : pm.queryIntentActivities(launcher, 0)) {
            final String pkg = app.activityInfo.packageName;
            if (!seen.add(pkg)) {
                continue;
            }
            pool.execute(() -> {
                try {
                    String line = pkg + " " + encode(app.loadIcon(pm));
                    synchronized (out) {
                        out.println(line);
                    }
                } catch (Exception e) {
                    // One broken icon must not stop the rest.
                }
            });
        }
        pool.shutdown();
        pool.awaitTermination(1, TimeUnit.MINUTES);
        out.flush();
    }

    /** app_process has no app context, so borrow the system one. */
    private static Context systemContext() throws Exception {
        Class<?> threadClass = Class.forName("android.app.ActivityThread");
        Object thread = threadClass.getMethod("systemMain").invoke(null);
        return (Context) threadClass.getMethod("getSystemContext").invoke(thread);
    }

    private static String encode(Drawable icon) {
        Bitmap bitmap = Bitmap.createBitmap(SIZE, SIZE, Bitmap.Config.ARGB_8888);
        icon.setBounds(0, 0, SIZE, SIZE);
        icon.draw(new Canvas(bitmap));

        ByteArrayOutputStream png = new ByteArrayOutputStream();
        bitmap.compress(Bitmap.CompressFormat.PNG, 100, png);
        return Base64.encodeToString(png.toByteArray(), Base64.NO_WRAP);
    }
}
