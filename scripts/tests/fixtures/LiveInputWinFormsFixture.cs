using System;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;

// A real managed Button receives the input. Only its actual events write the result.
internal sealed class LiveInputWinFormsFixture : Form
{
    [DllImport("user32.dll")]
    private static extern bool SetProcessDpiAwarenessContext(IntPtr value);

    [STAThread]
    private static int Main(string[] args)
    {
        int screenIndex;
        if (args.Length != 3 || !int.TryParse(args[1], out screenIndex)) return 2;
        try
        {
            SetProcessDpiAwarenessContext(new IntPtr(-4));
        }
        catch (EntryPointNotFoundException)
        {
            // A system without per-monitor-v2 still has the WinForms test surface.
        }

        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Screen[] screens = Screen.AllScreens;
        if (screenIndex < 0 || screenIndex >= screens.Length) return 2;
        using (var form = new LiveInputWinFormsFixture(args[0], screens[screenIndex], args[2]))
        {
            Application.Run(form);
        }
        return 0;
    }

    private readonly Button action = new Button
    {
        Text = "Live input test",
        Location = new Point(32, 32),
        Size = new Size(160, 32),
    };
    private readonly string statePath;
    private int clicks;
    private int dragEdges;
    private MouseButtons pressed;

    private LiveInputWinFormsFixture(string readyPath, Screen screen, string statePath)
    {
        this.statePath = statePath;
        Text = "Otter Live WinForms fixture";
        ClientSize = new Size(340, 160);
        FormBorderStyle = FormBorderStyle.FixedSingle;
        MaximizeBox = false;
        StartPosition = FormStartPosition.Manual;
        Rectangle work = screen.WorkingArea;
        Location = new Point(work.Left + Math.Max(0, (work.Width - Width) / 2),
            work.Top + Math.Max(0, (work.Height - Height) / 2));
        Controls.Add(action);

        action.Click += (_, __) => { clicks++; WriteState(); };
        action.MouseDown += (_, e) => RecordButtonEdge(e.Button, true);
        action.MouseUp += (_, e) => RecordButtonEdge(e.Button, false);
        action.MouseMove += (_, __) =>
        {
            if ((pressed & MouseButtons.Left) == 0) return;
            dragEdges++;
            WriteState();
        };
        Shown += (_, __) =>
        {
            WriteState();
            WriteReady(readyPath);
        };
    }

    protected override bool ShowWithoutActivation { get { return true; } }

    private void RecordButtonEdge(MouseButtons button, bool down)
    {
        if (button != MouseButtons.Left) return;
        if (down) pressed |= button; else pressed &= ~button;
        dragEdges++;
        WriteState();
    }

    private void WriteReady(string path)
    {
        int y = action.ClientSize.Height / 2;
        Point click = PointToClient(action.PointToScreen(new Point(action.ClientSize.Width / 2, y)));
        // Move beyond click jitter, but keep the mouse message on the same managed control.
        Point drag = PointToClient(action.PointToScreen(new Point(action.ClientSize.Width - 4, y)));
        File.WriteAllText(path, string.Format(CultureInfo.InvariantCulture,
            "{{\"hwnd\":\"0x{0:x}\",\"targets\":{{\"action\":[{1:R},{2:R}],\"dragEnd\":[{3:R},{4:R}]}}}}",
            Handle.ToInt64(), (double)click.X / ClientSize.Width, (double)click.Y / ClientSize.Height,
            (double)drag.X / ClientSize.Width, (double)drag.Y / ClientSize.Height));
    }

    private void WriteState()
    {
        File.WriteAllText(statePath, string.Format(CultureInfo.InvariantCulture,
            "{{\"clicks\":{0},\"dragEdges\":{1},\"pressedMouseButtons\":{2}}}",
            clicks, dragEdges, (int)pressed));
    }
}
