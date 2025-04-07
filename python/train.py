import numpy as np

from bevy_env import BevyRLEnv

from stable_baselines3 import SAC

env = BevyRLEnv()
print("restart environment")
env.reset(seed=1)

model = SAC("MlpPolicy", env, verbose=1)

print("start training")
model.learn(total_timesteps=50000, log_interval=10)
model.save("revy_rl")
