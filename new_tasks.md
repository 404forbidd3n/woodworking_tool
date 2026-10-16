


A simple woodworking app similar to flatma.


Scopes or features of this simple app.


1. A 3d space in which i can add, edit, delete furniture objects.
2. A simple sidebar in which i can add time of furnitures :
        - Cabinet with drawer or drawers (specify number of drawers and arange them equily inside the cabinet based on the hight)
        - A drawer with one or two door
        - A drawer with a door which opens upside down
        - A cabiner or drawer with sliding doors
        - A simple table with 4 legs
        - A custom cabiner/drawer - > basically just a placeholder box with custom dimentions
3. Properties under the sidebar in which we have the types of each cabinet or drawer, here i need to specify the width, height and depth of each of them and reference them in the 3d space
4. I need to be able to move the objects in the 3d space as i like


Stage 2:

- add another section in which i can pick colors or materials for each cabinet (as a plus it would be ideal if i can pick different collors for different parts of the cabinet, each wall of the cabinet for example to be a different collor, and also each face front, if its a drawer or a door i should be abble to pick the color of the front face)

- add another component to the list which is the table tops, basically i want different tabletops and different drawer tops, ith like the main kitchen counter, that usually is made out of a different material and also its alot more thicker than the regular wood for the drawer faces and sides

- in the sidebar with all these components, try to rename them to be more accurate and simply named basically, try to google search all the types of drawers and cabinates i might need, maybe add a 2d icon for each to be a bit more clear on what i want to add to the stage, do not remove any functionality which give me the ability to properly edit each of them, i still need to be able to edit the number of doors or drawers on each component

- i want to be able to multiselect multiple items in the stage to move them togheater, pick with CTRL+CLICK to pick them, and move them

- i want the ability to open a properties like window when i right click and hit properties on each component, the properties windows should contain ALL the details and i should be able to edit all the details of that given drawer/object

- i also want to add a few kitchen apliance 3d models, like coffee machine, fridge, dishwasher, electric oven, etc . i will need a whole new tab which contains all of thees, the tab should be called something like : add appliance

- scrool speed should be tuned down a bit, i think the scroll speed is a bit high right now

- menu in the left looks a bit better, but some text seems a bit shaded, i want the text for each Dropdown to be bald, the font size a bit bigger and the collor black, so i can properly see each tab


Stage 3

I want to add a separate button which opens a completly new window which will have a 2D canvas, i want this functionality to sit in a separate subfolder or area of the project, and i also want it to be well documented in the code with comments, i will describe bellow what i will want here :


- in this window i would like to add the dimensions for a raw board of plywood or wood -> this should be the total dimensions of the supplyed wood -> from there i want to add the number of doors and shelves and tops and bottoms and other parts which i want to cut

- the program should show me all the cuts i want to make on the 2d canvas, which will show the full board, and where should i cut to waste as little material as possible

- the canvas should clearly show the width and hight with big bold numbers, and the areas where the cuts will happen should be highlited with different collors

- i want to have a list in the left panel to basically add the number and the dimensions of the sub-boards i want to cut

- in the end after i hit another button, lets say "estimate" i want to see how many board i need to order for all the components i am trying to cut




Stage 4:

1. I want to split all the furniture types in 2 sub categories following this structure:

        Floor : 
                - Cabinet with drawer or drawers (specify number of drawers and arange them equily inside the cabinet based on the hight)
                - A drawer with one or two door
                - A cabiner or drawer with sliding doors
                - A simple table with 4 legs
                - A custom cabiner/drawer - > the initial placeholder type should be change in a cabinet with only shelves with no door, the number of shelves should be editable as the number of drawers for the drawer

        Wall mounted :
                - A drawer with one or two door
                - A drawer with a door which opens upside down
                - A drawerbinet -> this is basically a cabinet with drawers instead of normal doors

The floor items should have an option for plastic legs, or metal legs, those legs are usually with screws which makes the cabinet adjustable in hight, i want all the drawers and cabbinets to have legs of this type attached by default, i should be able to remove them manually afterwards (maybe like a select box for legs, metal 100mm, metal 150mm plastic, wood, none, etc -> fill in some of them for me)

All the mounted ones should have a lower depth by default, 350mm should be enough


2. the ability to UNDO, somethins i drag something by mystake and i cannot undo, i just want a very basic for of UNDO/DO -> not to many items in the history, lets say 5-10 UNDO's should be enough

3. Fix the UI for the properties area, some text is still not visible, with black and bold text to the dropdown buttons, simillar with the initial menu

4. The ability to save and load specific .json configs, i want to have multiple projects, saved as JSON files, so i need to name files, open specific files, load, save, etc . Make the menu bar with a LOAD select file, and a save -> with file name . something like that.

5. I want the width measurement of each cabinet and drawer to be written in bald blac text with arros pointing to its with left to right underneath each of them, the text does not need to be too large so it will not interfere to much in the visual space.

6. I want to add a few basic edits to the room itself, like adding a door place , a window place, and its positions basically, leave the room a bit editable for this


        