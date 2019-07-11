using Interop;
using Microsoft.Win32;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;

namespace viewsvg_skia.net
{
    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        SvgView view_;

        public MainWindow()
        {
            InitializeComponent();

            this.SourceInitialized += MainWindow_SourceInitialized;
            this.Closed += MainWindow_Closed;
            this.Loaded += MainWindow_Loaded;
            this.SizeChanged += MainWindow_SizeChanged;            
        }

        private void MainWindow_SizeChanged(object sender, SizeChangedEventArgs e)
        {
            if (this.IsLoaded)
            {
                RenderImage();
            }
        }

        private void MainWindow_SourceInitialized(object sender, EventArgs e)
        {
            SvgView.Initialize();
        }

        private void MainWindow_Closed(object sender, EventArgs e)
        {
            SvgView.Terminate();
        }

        private void MainWindow_Loaded(object sender, RoutedEventArgs e)
        {
            string filePath = System.IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory,
                @"..\..\..\..\..\..\capi\skiatests\drawing-7.svg");
            LoadFile(filePath);
            RenderImage();
        }

        private void RenderImage()
        {                      
            if (view_ != null)
            {
                Debug.Print("SVG size:  {0} x {1}", view_.SvgWidth, view_.SvgHeight);

                uint dw = (uint)this.MainCanvas.ActualWidth;
                uint dh = (uint)this.MainCanvas.ActualHeight;
                BitmapSource bitmapSource = view_.DrawImage(0.0, 0.0, view_.SvgWidth, view_.SvgHeight, dw, dh);
                if (bitmapSource != null)
                {
                    Image image = new Image();
                    image.Source = bitmapSource;                    
                    this.MainCanvas.Children.Clear();
                    this.MainCanvas.Children.Add(image);

                    SaveBitmapSourceToFile(bitmapSource);
                }           
            }
        }

        private void AppExit_Click(object sender, RoutedEventArgs e)
        {
            this.Close();
        }

        private void FileOpen_Click(object sender, RoutedEventArgs e)
        {
            OpenFileDialog openFileDialog = new OpenFileDialog();
            openFileDialog.Filter = "SVG files (*.svg)|*.svg|All files (*.*)|*.*";
            if (openFileDialog.ShowDialog() == true)
            {
                LoadFile(openFileDialog.FileName);
                RenderImage();
            }
        }

        private void LoadFile(string filePath)
        {
            if (view_ != null)
            {
                view_.Dispose();
                view_ = null;
            }

            view_ = SvgView.LoadFile(filePath);
            if (view_ != null)
            {
                string exportPath = System.IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "out.svg");
                view_.Export(exportPath, false);
            }

        }

        private void SaveBitmapSourceToFile(BitmapSource source)
        {
            string exportPath = System.IO.Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "out.png");
            using (var fileStream = new FileStream(exportPath, FileMode.Create))
            {
                BitmapEncoder encoder = new PngBitmapEncoder();
                encoder.Frames.Add(BitmapFrame.Create(source));
                encoder.Save(fileStream);
            }
        }

    }
}
