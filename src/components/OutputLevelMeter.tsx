import { LevelMeter } from "./LevelMeter";

export function OutputLevelMeter(props: Omit<React.ComponentProps<typeof LevelMeter>, "label">) {
  return <LevelMeter label="Output level" {...props} />;
}
