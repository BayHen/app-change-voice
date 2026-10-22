import { LevelMeter } from "./LevelMeter";

export function InputLevelMeter(props: Omit<React.ComponentProps<typeof LevelMeter>, "label">) {
  return <LevelMeter label="Input level" {...props} />;
}
