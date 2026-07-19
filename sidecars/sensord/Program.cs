// sensord — sidecar capteurs + contrôle ventilateurs pour PureRGB.
// Émet sur stdout une ligne JSON par tick (2 s) : températures, ventilateurs,
// charges, contrôles. Accepte sur stdin une commande JSON par ligne :
//   {"cmd":"set","id":"/lpc/nct6798d/0/control/0","value":55}
//   {"cmd":"reset","id":"/lpc/nct6798d/0/control/0"}
// set = pilotage logiciel du canal (0-100 %), reset = rend la main au BIOS.
// S'arrête proprement quand stdin se ferme (parent mort).
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

// Index id -> capteur, reconstruit à chaque tick (matériel stable en pratique).
var sensorIndex = new Dictionary<string, ISensor>();
var indexLock = new object();

// Commandes stdin : une ligne JSON par commande ; EOF = fin propre.
var stop = new CancellationTokenSource();
_ = Task.Run(() =>
{
    try
    {
        string? line;
        while ((line = Console.In.ReadLine()) != null)
        {
            line = line.Trim();
            if (line.Length == 0) continue;
            try
            {
                var doc = JsonDocument.Parse(line);
                var root = doc.RootElement;
                var cmd = root.GetProperty("cmd").GetString();
                var id = root.GetProperty("id").GetString() ?? "";
                ISensor? sensor;
                lock (indexLock) { sensorIndex.TryGetValue(id, out sensor); }
                if (sensor?.Control is null)
                {
                    Console.Error.WriteLine($"control introuvable: {id}");
                    continue;
                }
                if (cmd == "set" && root.TryGetProperty("value", out var v))
                {
                    var pct = Math.Clamp(v.GetSingle(), 0f, 100f);
                    sensor.Control.SetSoftware(pct);
                }
                else if (cmd == "reset")
                {
                    sensor.Control.SetDefault();
                }
            }
            catch (Exception e)
            {
                Console.Error.WriteLine($"commande invalide: {e.Message}");
            }
        }
    }
    catch { }
    stop.Cancel();
});

var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower };

while (!stop.IsCancellationRequested)
{
    var sensors = new List<object>();
    var newIndex = new Dictionary<string, ISensor>();
    foreach (var hw in computer.Hardware)
    {
        hw.Update();
        foreach (var sub in hw.SubHardware)
        {
            sub.Update();
            Collect(sub, sensors, newIndex);
        }
        Collect(hw, sensors, newIndex);
    }
    lock (indexLock) { sensorIndex = newIndex; }
    Console.WriteLine(JsonSerializer.Serialize(new { sensors }, options));
    Console.Out.Flush();
    try { Task.Delay(2000, stop.Token).Wait(); } catch { break; }
}

computer.Close();
return 0;

static void Collect(IHardware hw, List<object> sensors, Dictionary<string, ISensor> index)
{
    foreach (var s in hw.Sensors)
    {
        if (s.Value is null) continue;
        // Seuls les types utiles aux courbes, à l'affichage et au pilotage.
        if (s.SensorType is not (SensorType.Temperature or SensorType.Fan
            or SensorType.Load or SensorType.Control or SensorType.Power)) continue;
        var id = s.Identifier.ToString();
        index[id] = s;
        sensors.Add(new
        {
            id,
            hardware = hw.Name,
            name = s.Name,
            type = s.SensorType.ToString(),
            value = Math.Round(s.Value.Value, 1),
            // true si le canal accepte un pilotage logiciel (ventilateur mobo).
            controllable = s.Control is not null,
        });
    }
}
