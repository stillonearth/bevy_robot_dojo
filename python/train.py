import numpy as np

from bevy_env import BevyRLEnv

from stable_baselines3 import SAC

env = BevyRLEnv()

model = SAC("MlpPolicy", env, verbose=1)
model.learn(total_timesteps=50000, log_interval=10)
model.save("sac_pendulum")
