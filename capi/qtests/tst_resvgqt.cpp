#include <QString>
#include <QPainter>
#include <QtTest>

#include <ResvgQt.h>

class ResvgQtTests : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void test_parseFile();
    void test_parseInvalidFile();
    void test_emptyFile();

    void test_renderFile();

    void test_imageSize();
    void test_imageViewBox();
    void test_imageBoundingBox();
    void test_elementExists();
    void test_transformForElement();
};

static QString localPath(const QString &fileName)
{
    return QString("%1/%2").arg(SRCDIR).arg(fileName);
}

void ResvgQtTests::test_parseFile()
{
    ResvgRenderer render(localPath("test.svg"));
    QVERIFY(render.isValid());
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.defaultSize(), QSize(200, 200));
}

void ResvgQtTests::test_parseInvalidFile()
{
    ResvgRenderer render(localPath("invalid.svg"));
    QVERIFY(!render.isValid());
    QVERIFY(render.isEmpty());
}

void ResvgQtTests::test_emptyFile()
{
    ResvgRenderer render(localPath("empty.svg"));
    QVERIFY(render.isValid());
    QVERIFY(render.isEmpty());
}

void ResvgQtTests::test_renderFile()
{
#ifdef LOCAL_BUILD
    ResvgRenderer render(localPath("test.svg"));
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.defaultSize(), QSize(200, 200));

    QImage img(render.defaultSize(), QImage::Format_ARGB32);
    img.fill(Qt::transparent);

    QPainter p(&img);
    render.render(&p);
    p.end();

    img.save("test_renderFile.png");

    QCOMPARE(img, QImage(localPath("test_renderFile_result.png")));
#endif
}

void ResvgQtTests::test_imageSize()
{
    ResvgRenderer render(localPath("vb.svg"));
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.defaultSize(), QSize(200, 400));
}

void ResvgQtTests::test_imageViewBox()
{
    ResvgRenderer render(localPath("vb.svg"));
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.viewBox(), QRect(50, 100, 200, 400));
}

void ResvgQtTests::test_imageBoundingBox()
{
    ResvgRenderer render(localPath("test.svg"));
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.boundingBox().toRect(), QRect(20, 20, 160, 160));
}

void ResvgQtTests::test_elementExists()
{
    ResvgRenderer render(localPath("test.svg"));
    QVERIFY(!render.isEmpty());

    // Existing element.
    QVERIFY(render.elementExists("circle1"));

    // Non-existing element.
    QVERIFY(!render.elementExists("invalid"));

    // Non-renderable elements.
    QVERIFY(!render.elementExists("rect1"));
    QVERIFY(!render.elementExists("rect2"));
    QVERIFY(!render.elementExists("patt1"));
}

void ResvgQtTests::test_transformForElement()
{
    ResvgRenderer render(localPath("test.svg"));
    QVERIFY(!render.isEmpty());
    QCOMPARE(render.transformForElement("circle1"), QTransform(2, 0, 0, 2, 0, 0));
    QCOMPARE(render.transformForElement("invalid"), QTransform());
}

QTEST_APPLESS_MAIN(ResvgQtTests)

#include "tst_resvgqt.moc"
