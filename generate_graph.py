from pyvis.network import Network
import json 
net = Network('1500px', '1500px')

# load json
f = open('graph.json')

graph = json.load(f)
for node in graph["nodes"]:
    net.add_node(int(node["id"]), label=node["label"])
    
    
for edge in graph["edges"]:
    net.add_edge(int(edge["source"]), int(edge["target"]))
net.toggle_physics(True)
net.show('nx.html')