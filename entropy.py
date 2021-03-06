#!/usr/bin/env python3

#   Copyright 2021 Robin Marchart
#
#      Licensed under the Apache License, Version 2.0 (the "License");
#      you may not use this file except in compliance with the License.
#      You may obtain a copy of the License at
#
#          http://www.apache.org/licenses/LICENSE-2.0
#
#      Unless required by applicable law or agreed to in writing, software
#      distributed under the License is distributed on an "AS IS" BASIS,
#      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#      See the License for the specific language governing permissions and
#      limitations under the License.

import sys

import matplotlib.pyplot as plt
import numpy as np

arr_y = np.load(sys.argv[1])
arr_x = np.arange(arr_y[0], arr_y[0] + len(arr_y) - 1, 1)
plt.bar(arr_x, arr_y[1:], 1)
plt.xlabel("result")
plt.ylabel("num events")
plt.savefig(sys.argv[2], format="png", transparent=False, backend="cairo")
