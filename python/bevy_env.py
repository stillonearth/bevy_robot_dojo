import requests
import numpy as np
import json

import gymnasium as gym

API_STEP = "http://127.0.0.1:7878/step"
API_STATE = "http://127.0.0.1:7878/state"
API_RESET = "http://127.0.0.1:7878/reset"

OBSERVATION_SIZE = 69
ACTION_SIZE = 10


class BevyRLEnv(gym.Env):

    def __init__(self):

        self.observation_space = gym.spaces.Box(
            low=-np.inf, high=np.inf, shape=(OBSERVATION_SIZE,), dtype=float
        )
        self.action_space = gym.spaces.Box(
            low=-10.0, high=10.0, shape=(ACTION_SIZE,), dtype=float
        )

    def get_obs(self):
        state = requests.get(API_STATE).json()
        if state and ("transforms" in state) and ("joint_angles" in state):
            transforms = np.concatenate(
                [
                    np.concatenate([t for t in state["transforms"]]),
                    state["joint_angles"],
                ]
            )
            return transforms
        else:
            return np.zeros(OBSERVATION_SIZE)

    def step_env(self, action):
        payload = json.dumps([{"action": json.dumps(list(action))}], indent=4)
        return requests.get(API_STEP, params={"payload": payload}).json()

    def step(self, action):
        obs = self.get_obs()
        step_data = self.step_env(action)[0]

        is_terminated = step_data["is_terminated"]
        reward = step_data["reward"]

        return obs, reward, is_terminated, False, {}

    def reset_env(self):
        requests.get(API_RESET)

    def reset(self, seed):
        self.reset_env()
        return self.get_obs(), {}

    def render():
        None
