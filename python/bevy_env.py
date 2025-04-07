import requests
import numpy as np
import json

import gymnasium as gym

API_STEP = "http://127.0.0.1:7879/step"
API_STATE = "http://127.0.0.1:7879/state"
API_RESET = "http://127.0.0.1:7879/reset"

OBSERVATION_SIZE = 69
ACTION_SIZE = 16


class BevyRLEnv(gym.Env):

    def __init__(self):

        self.observation_space = gym.spaces.Box(
            low=-np.inf, high=np.inf, shape=(OBSERVATION_SIZE,), dtype=float
        )
        self.action_space = gym.spaces.Box(
            low=-5.0, high=5.0, shape=(ACTION_SIZE,), dtype=float
        )

    def get_obs(self):
        state = requests.get(API_STATE).json()
        transforms = np.concatenate(
            [np.concatenate([t for t in state["transforms"]]), state["joint_angles"]]
        )
        return transforms

    def step_env(self, action):
        payload = json.dumps([{"action": json.dumps(list(action))}], indent=4)
        return requests.get(API_STEP, params={"payload": payload}).json()

    def step(self, action):
        print("env.step start", action)
        obs = self.get_obs()
        step_data = self.step_env(action)[0]
        print("env.step end")
        is_terminated = step_data["is_terminated"]
        reward = step_data["reward"]

        

        return obs, reward, is_terminated, False, {}

    def reset_env(self):
        requests.get(API_RESET)

    def reset(self, seed):
        print("env.reset start")
        self.reset_env()

        print("env.reset end ")

        return self.get_obs(), {}

    def render():
        None
