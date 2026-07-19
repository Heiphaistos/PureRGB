// sensord — sidecar capteurs pour PureRGB.
// Émet sur stdout une ligne JSON par tick (2 s) : températures, ventilateurs,
// charges. S'arrête proprement quand stdin se ferme (parent mort).
// Licence lib : LibreHardwareMonitorLib (MPL 2.0), utilisée telle quelle.

using System.Text.Json;
using LibreHardwareMonitor.Hardware;

var computer = new Computer
{
    IsCpuEnabled = true,
    IsGpuEnabled = true,
    IsMotherboardEnabled = true,
    IsControllerEnabled = true,
    IsStorageEnabled = true,
};
computer.Open();

// Fin propre quand le parent ferme stdin (PureRGB quitte).
var stop = new CancellationTokenSource();
_ = Task.Run(() =>
{
    try { Console.In.ReadToEnd(); } catch { }
    stop.Cancel();
});

var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };

while (!stop.IsCancellationRequested)
{
    var sensors = new List<object>();
    foreach (var hw in computer.Hardware)
    {
        hw.Update();
        foreach (var sub in hw.SubHardware)
        {
            sub.Update();
            Collect(sub, sensors);
        }
        Collect(hw, sensors);
    }
    Console.WriteLine(JsonSerializer.Serialize(new { sensors }, options));
    Console.Out.Flush();
    try { Task.Delay(2000, stop.Token).Wait(); } catch { break; }
}

computer.Close();
return 0;

static void Collect(IHardware hw, List<object> sensors)
{
    foreach (var s in hw.Sensors)
    {
        if (s.Value is null) continue;
        // Seuls les types utiles aux courbes et à l'affichage.
        if (s.SensorType is not (SensorType.Temperature or SensorType.Fan
            or SensorType.Load or SensorType.Control or SensorType.Power)) continue;
        sensors.Add(new
        {
            id = s.Identifier.ToString(),
            hardware = hw.Name,
            name = s.Name,
            type = s.SensorType.ToString(),
            value = Math.Round(s.Value.Value, 1),
        });
    }
}
