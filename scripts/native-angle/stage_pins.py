"""已审阅 D3D11/Clang profile 的固定 Git 对象与工具归档；不自动解析最新版。"""

# key、src 内挂载点、commit、tree。缺省 gitlink 不下载，逐项记入输出清单。
SOURCES = (
    ("angle", "", "ded782bca9d5f165d1c4a70124cdc5384043a8b3", "a6899d5ff36cff7d8b05ceb886be7931bbcca4ff"),
    ("build", "build", "8a46ede8ea86151737d9c284f82d238b3c701fa7", "ee8c6f5518fa56c29f312bd2b2bfedc9e88c266a"),
    ("buildtools", "buildtools", "5e7f45b2c8d0acf21ee52a32bb6ec83d4620eb59", "b9417aa0a6f1878edc0a8541f819417f5e6edecd"),
    ("jsoncpp", "third_party/jsoncpp", "f62d44704b4da6014aa231cfc116e7fd29617d2a", "b569211ecd2584cc18ab5a4bc316317990fcad95"),
    ("jsoncpp-source", "third_party/jsoncpp/source", "42e892d96e47b1f6e29844cc705e148ec4856448", "c9b5ffc95a60647ddfba8a148daa86d0235df093"),
    ("testing", "testing", "d1c721eb8a52c213b4fe2041bbc15edc009c3e2d", "4a0c88479d97d45dbb54ca3076a87f45c4cb00aa"),
    ("abseil", "third_party/abseil-cpp", "5e42a36a85a252d8cdee6c39661d2bfd9883fd5c", "8ec47e33aa935ca4d3ccd7cc67191a9d34937981"),
    ("astc", "third_party/astc-encoder/src", "2319d9c4d4af53a7fc7c52985e264ce6e8a02a9b", "1dec30e5746d8943526e18729318bf26efe57476"),
    ("rapidjson", "third_party/rapidjson/src", "24b5e7a8b27f42fa16b96fc70aade9106cf7102f", "42220faaa684c14bfc42aeebc6d5006fb6c51eff"),
    ("zlib", "third_party/zlib", "89921383f1f51331f48a2dc843403779770ea3a9", "53ad85728fb29044ca9c65cde5af80d619b6dbc7"),
    ("clang-scripts", "tools/clang", "3e520fe28bbeed4a87f5e575272933d359915989", "245bf650e1cd282c9cd60e5e266cc9d02b72a348"),
    ("vulkan-headers", "third_party/vulkan-headers/src", "f6a6f7ab165cedbfa2a7d0c93fe27a2d01ce09c8", "22e91ab4c76b9566b98cae31a77207f73153702c"),
    ("spirv-tools", "third_party/spirv-tools/src", "2ec8457ab33d539b6f1fecc998360c0b8b05ed4f", "71935e02ec8950c908ed613a2ebf2b41da7bee9b"),
    ("spirv-headers", "third_party/spirv-headers/src", "53b334f4a5f18fb1f2d367baeba8b43e2eb0b62b", "26464d5804e6cb3048d53bea208144ee2cfcbb1a"),
    ("tools-win", "tools/win", "d16e6b55b2bd699735919d8a13a55ff284086603", "e5a247c1dcca47367f144b59afba24b7197aecf6"),
)
ARTIFACTS = {
    "gn": "f71fc4fd47d741a2f368f25727af995532eac909a5d8b8fdc778f477767b8a93",
    "ninja": "0cf1cb1b9d2b8d2d14a9fd38b984f17dd853115db682880b21ceee45e91deb50",
    "clang": "7fa1aa9bf477f565687a2f516bf75ff6bfdd13da9b36bf521a75ac8ae2761d6a",
    "chromium-rc": "903769d98df8a005c79d52d7089fe34f23c0b7ada4cd0c42b6063526a5593e6a",
}
